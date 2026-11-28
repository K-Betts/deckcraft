//! The maths edition's subcommands: `fidelity` (compare a deck before and after a save), `bank
//! check` and `curriculum check` (the Get Ready question bank and a school's curriculum file).

use super::{Ordering, STDOUT_CLOSED, Value, help_page, is_help, json, stdout_line};

/// Run subcommand `name` (`fidelity`, `bank` or `curriculum`).
pub(super) fn run(name: &str, args: &[String]) -> Result<(), String> {
    match name {
        "fidelity" => fidelity(args),
        "bank" => bank(args),
        _ => curriculum(args),
    }
}

/// `bank check FILE [--samples N]`: validate a question bank and print generated examples.
fn bank(args: &[String]) -> Result<(), String> {
    use deckcraft_engine::{latex, questions};
    if args.first().is_some_and(|a| is_help(a)) {
        outln!("{}", help_page("bank"));
        return Ok(());
    }
    if args.first().map(String::as_str) != Some("check") {
        return Err("bank needs a subcommand: bank check FILE".into());
    }
    let path = args.get(1).ok_or("bank check needs a FILE")?;
    let samples = match args.iter().position(|a| a == "--samples") {
        Some(i) => {
            args.get(i + 1).and_then(|n| n.parse::<u64>().ok()).filter(|n| (1..=20).contains(n)).ok_or("--samples needs a number from 1 to 20")?
        }
        None => 3,
    };
    let text = String::from_utf8(std::fs::read(path).map_err(|e| format!("{path}: {e}"))?).map_err(|_| format!("{path} is not text"))?;
    let bank = questions::parse_loose(&text).map_err(|e| format!("PROBLEM: {e}"))?;
    outln!("OK: {} fixed questions and {} templates", bank.questions.len(), bank.templates.len());
    for t in &bank.templates {
        outln!("\n{} · {} · {} · level {}", t.id, t.topic, t.subtopic, t.difficulty);
        for w in t.lint() {
            outln!("  CHECK THIS: {w}");
        }
        for n in 0..samples {
            let q = t.instantiate(n * 7 + 1)?;
            outln!("  Q: {}\n  A: {}", latex::plain(&q.text), latex::plain(&q.answer));
        }
    }
    Ok(())
}

/// `curriculum check FILE`: read a curriculum file and show the question-bank skills each
/// objective is matched to, flagging objectives nothing in the bank fits.
fn curriculum(args: &[String]) -> Result<(), String> {
    use deckcraft_engine::{curriculum, questions};
    if args.first().is_some_and(|a| is_help(a)) {
        outln!("{}", help_page("curriculum"));
        return Ok(());
    }
    if args.first().map(String::as_str) != Some("check") {
        return Err("curriculum needs a subcommand: curriculum check FILE".into());
    }
    let path = args.get(1).ok_or("curriculum check needs a FILE")?;
    let text = String::from_utf8(std::fs::read(path).map_err(|e| format!("{path}: {e}"))?).map_err(|_| format!("{path} is not text"))?;
    let cur = curriculum::parse(&text).map_err(|e| format!("PROBLEM: {e}"))?;
    let bank = questions::starter()?.instantiate(0);
    let m = curriculum::Matcher::new(bank.skills());
    let name = |key: &str| {
        m.skills
            .iter()
            .find(|s| s.key == key)
            .map(|s| format!("{} ({})", s.name, if s.year.is_empty() { "general" } else { &s.year }))
            .unwrap_or_default()
    };
    let (mut total, mut unmatched, mut marked) = (0, 0, 0);
    for course in &cur.courses {
        outln!("\n== {} ({} units)", course.name(), course.units.len());
        for u in &course.units {
            outln!("  {} {}", if u.start.is_empty() { "          " } else { &u.start }, u.title);
            for (o, prior) in u.objectives.iter().map(|o| (o, "")).chain(u.prior.iter().map(|o| (o, "prior: "))) {
                total += 1;
                // Prior learning is matched on its own words, objectives with their unit's too.
                let context = if prior.is_empty() { u.title.as_str() } else { "" };
                let keys = m.skills_for(o, context, &course.year, &course.route);
                // How well each fits (0 to 1), when the words chose them rather than the file.
                let text = if o.search.trim().is_empty() { &o.text } else { &o.search };
                let scores: Vec<(usize, f64)> = if o.skills.is_empty() { m.best_in(text, context, &course.year, &course.route, 3) } else { vec![] };
                let fit = |k: &String| scores.iter().find(|(i, _)| &m.skills[*i].key == k).map(|(_, sc)| format!(" {sc:.2}")).unwrap_or_default();
                if o.unmatched {
                    marked += 1;
                    outln!("    UNMATCHED {prior}{}", o.text);
                } else if keys.is_empty() {
                    unmatched += 1;
                    outln!("    NO MATCH  {prior}{}", o.text);
                } else {
                    outln!(
                        "    {prior}{}\n        -> {}",
                        o.text,
                        keys.iter().map(|k| format!("{}{}", name(k), fit(k))).collect::<Vec<_>>().join("  |  ")
                    );
                }
            }
        }
    }
    outln!("\nOK: {} courses, {total} objectives, {unmatched} with no matching skill, {marked} marked unmatched", cur.courses.len());
    Ok(())
}

/// Compare two imported presentations by slide position and print a JSON difference report.
fn fidelity(args: &[String]) -> Result<(), String> {
    if args.first().is_some_and(|a| is_help(a)) {
        outln!("{}", help_page("fidelity"));
        return Ok(());
    }
    let positional: Vec<&str> = args.iter().filter(|a| !a.starts_with("--")).map(String::as_str).collect();
    if args.iter().any(|a| a.starts_with("--")) {
        return Err("fidelity: unexpected option".into());
    }
    let [original_path, saved_path] = positional.as_slice() else {
        return Err("fidelity needs ORIGINAL and SAVED files".into());
    };
    let result = fidelity_report(original_path, saved_path)?;
    let output = serde_json::to_string_pretty(&result).map_err(|e| format!("fidelity report: {e}"))?;
    outln!("{output}");
    Ok(())
}

fn fidelity_report(original_path: &str, saved_path: &str) -> Result<Value, String> {
    let original_bytes = std::fs::read(original_path).map_err(|e| format!("{original_path}: {e}"))?;
    let saved_bytes = std::fs::read(saved_path).map_err(|e| format!("{saved_path}: {e}"))?;
    let original = deckcraft_engine::cmd::file::open_presentation(original_path, &original_bytes).map_err(|e| e.to_string())?;
    let saved = deckcraft_engine::cmd::file::open_presentation(saved_path, &saved_bytes).map_err(|e| e.to_string())?;
    let changes = deckcraft_engine::compare::diff(&original, &saved, true);
    Ok(json!({
        "version": 1,
        "comparison": "slide-model",
        "original": original_path,
        "saved": saved_path,
        "slides": {
            "original": original.slides.len(),
            "saved": saved.slides.len(),
        },
        "hasSlideChanges": original.slides.len() != saved.slides.len() || !changes.is_empty(),
        "changes": changes,
    }))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use deckcraft_engine::Session;

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn fidelity_accepts_help() {
        assert!(fidelity(&args(&["--help"])).is_ok());
    }

    #[test]
    fn fidelity_report_identifies_notes_changes() {
        let dir = std::env::temp_dir().join(format!("deckcraft-cli-fidelity-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let original_path = dir.join("original.pptx");
        let saved_path = dir.join("saved.pptx");
        let mut s = Session::with_new();
        let original = deckcraft_engine::cmd::file::save_bytes(&s.doc().unwrap().doc, "pptx").unwrap();
        std::fs::write(&original_path, original).unwrap();
        s.execute("slide.notes", &json!({"text": "Edited speaker notes"})).unwrap();
        let saved = deckcraft_engine::cmd::file::save_bytes(&s.doc().unwrap().doc, "pptx").unwrap();
        std::fs::write(&saved_path, saved).unwrap();

        let report = fidelity_report(&original_path.to_string_lossy(), &saved_path.to_string_lossy()).unwrap();
        assert_eq!(report["version"], 1);
        assert_eq!(report["comparison"], "slide-model");
        assert_eq!(report["hasSlideChanges"], true);
        assert_eq!(report["slides"]["original"], 1);
        assert_eq!(report["slides"]["saved"], 1);
        let details = report["changes"][0]["details"].as_array().unwrap();
        assert!(details.iter().any(|detail| detail == "notes"), "{details:?}");
        assert!(details.iter().any(|detail| detail == "notes page"), "{details:?}");

        std::fs::remove_file(original_path).unwrap();
        std::fs::remove_file(saved_path).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn fidelity_rejects_missing_inputs_and_unknown_options() {
        assert!(fidelity(&[]).unwrap_err().contains("ORIGINAL and SAVED"));
        assert!(fidelity(&args(&["one.pptx", "two.pptx", "--strict"])).unwrap_err().contains("unexpected option"));
    }
}
