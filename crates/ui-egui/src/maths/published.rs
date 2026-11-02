//! What a private web build was published with from the Mac app (its curriculum, own questions,
//! subject packs and Teacher Mode tools; see `apps/deckcraft-web/build.rs`), merged into the
//! settings a browser already has.
//!
//! The rule: published material is taken when the browser has none, or when a newer version is
//! published; what a teacher made in the browser is kept otherwise. A newer version replaces the
//! published things the browser has (a question with the same id, the curriculum, the tool list)
//! and adds the rest, so questions a teacher added in the browser stay.

use serde_json::Value;

use crate::SlideApp;
use crate::maths::app::MathsUi;
use crate::maths::maths_tools::MathsToolsSettings;

/// Merge `published` (the build's JSON) into `app`. `new_version` is true the first time this
/// browser sees this version.
pub fn apply(app: &mut SlideApp, published: &Value, new_version: bool) {
    merge(&mut app.ui.maths, &mut app.maths.tools, published, new_version);
}

/// [`apply`] on the two settings it changes.
pub fn merge(m: &mut MathsUi, tools: &mut MathsToolsSettings, published: &Value, new_version: bool) {
    let get = |k: &str| published.get(k).filter(|v| !v.is_null());
    if let Some(c) = get("curriculum").and_then(Value::as_str)
        && !c.trim().is_empty()
        && (new_version || m.curriculum.trim().is_empty())
    {
        m.curriculum = c.to_string();
    }
    if new_version {
        if let Some(qs) = get("myQuestions").and_then(|v| serde_json::from_value::<Vec<deckcraft_engine::questions::Question>>(v.clone()).ok()) {
            upsert(&mut m.my_questions, qs, |q| q.id.clone());
        }
        if let Some(ts) = get("myTemplates").and_then(|v| serde_json::from_value::<Vec<deckcraft_engine::template::Template>>(v.clone()).ok()) {
            upsert(&mut m.my_templates, ts, |t| t.id.clone());
        }
        for id in strings(get("removedQuestions")) {
            if !m.removed_questions.contains(&id) {
                m.removed_questions.push(id);
            }
        }
        m.packs.extend(strings(get("packs")));
        if let Some(t) = get("mathsTools").and_then(|v| serde_json::from_value::<MathsToolsSettings>(v.clone()).ok()) {
            *tools = t;
        }
    }
    for (key, field) in [("getReadyMode", &mut m.get_ready_mode), ("getReadyYear", &mut m.get_ready_year), ("getReadyRoute", &mut m.get_ready_route)]
    {
        if field.is_empty()
            && let Some(v) = get(key).and_then(Value::as_str)
        {
            *field = v.to_string();
        }
    }
    m.sanitize();
}

/// What the live school bank (the staff question-bank page's `live.json`) put into the settings
/// last time, so the next version can take it back out first: a question an editor deleted, or a
/// built-in one they put back, must not stay behind.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LiveState {
    pub questions: Vec<String>,
    pub removed: Vec<String>,
}

/// Make `m` match the live school bank `live`: undo what `prev` added, then add the bank's
/// questions and hide the built-ins it takes out. Returns what to remember for next time. A
/// question the teacher added themselves (any other id) is left alone.
pub fn apply_live(m: &mut MathsUi, live: &Value, prev: &LiveState) -> LiveState {
    m.my_questions.retain(|q| !prev.questions.contains(&q.id));
    m.removed_questions.retain(|id| !prev.removed.contains(id));
    let qs = live
        .get("myQuestions")
        .and_then(|v| serde_json::from_value::<Vec<deckcraft_engine::questions::Question>>(v.clone()).ok())
        .unwrap_or_default();
    let removed = strings(live.get("removedQuestions"));
    let state = LiveState { questions: qs.iter().map(|q| q.id.clone()).collect(), removed: removed.clone() };
    upsert(&mut m.my_questions, qs, |q| q.id.clone());
    for id in removed {
        if !m.removed_questions.contains(&id) {
            m.removed_questions.push(id);
        }
    }
    m.sanitize();
    state
}

/// Replace the items of `mine` that share an id with one of `theirs`, and add the rest of `theirs`.
fn upsert<T>(mine: &mut Vec<T>, theirs: Vec<T>, id: impl Fn(&T) -> String) {
    for t in theirs {
        match mine.iter().position(|m| id(m) == id(&t)) {
            Some(i) => mine[i] = t,
            None => mine.push(t),
        }
    }
}

fn strings(v: Option<&Value>) -> Vec<String> {
    v.and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(String::from).collect()).unwrap_or_default()
}

/// A short fingerprint (FNV-1a) of the published text, to tell one version from the next.
pub fn fingerprint(text: &str) -> String {
    let h = text.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3));
    format!("{h:016x}")
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use serde_json::json;

    use super::*;
    use crate::maths::maths_tools::MathsTool;

    fn question(id: &str, text: &str) -> Value {
        json!({"id": id, "topic": "Algebra", "difficulty": 2, "text": text, "answer": "1"})
    }

    fn published() -> Value {
        json!({
            "curriculum": "{\"name\":\"Published\"}",
            "myQuestions": [question("own-1", "Published one")],
            "removedQuestions": ["built-in-7"],
            "packs": ["maths"],
            "getReadyMode": "taught",
            "mathsTools": {"enabled": ["ruler", "protractor", "calculator"]},
            "examLibrary": true
        })
    }

    #[test]
    fn a_fresh_browser_takes_everything_published() {
        let (mut m, mut tools) = (MathsUi::default(), MathsToolsSettings::default());
        merge(&mut m, &mut tools, &published(), true);
        assert!(m.curriculum.contains("Published"));
        assert_eq!(m.my_questions.len(), 1);
        assert_eq!(m.removed_questions, ["built-in-7"]);
        assert!(m.packs.contains("maths"));
        assert_eq!(m.get_ready_mode, "taught");
        assert!(tools.is_enabled(MathsTool::Protractor) && tools.is_enabled(MathsTool::Calculator) && !tools.is_enabled(MathsTool::NumberLine));
    }

    #[test]
    fn the_same_version_again_keeps_what_the_teacher_changed() {
        let (mut m, mut tools) = (MathsUi::default(), MathsToolsSettings::default());
        merge(&mut m, &mut tools, &published(), true);
        m.curriculum = "{\"name\":\"Mine\"}".into();
        m.my_questions.push(serde_json::from_value(question("browser-1", "Made in the browser")).unwrap());
        tools.set_enabled(MathsTool::NumberLine, true);
        m.get_ready_mode = "choose".into();
        merge(&mut m, &mut tools, &published(), false);
        assert!(m.curriculum.contains("Mine"));
        assert_eq!(m.my_questions.len(), 2);
        assert!(tools.is_enabled(MathsTool::NumberLine));
        assert_eq!(m.get_ready_mode, "choose");
    }

    #[test]
    fn a_new_version_replaces_what_was_published_and_keeps_the_teachers_own() {
        let (mut m, mut tools) = (MathsUi::default(), MathsToolsSettings::default());
        merge(&mut m, &mut tools, &published(), true);
        m.my_questions.push(serde_json::from_value(question("browser-1", "Made in the browser")).unwrap());
        m.curriculum = "{\"name\":\"Mine\"}".into();
        let mut next = published();
        next["myQuestions"] = json!([question("own-1", "Published, edited"), question("own-2", "Published two")]);
        next["curriculum"] = json!("{\"name\":\"Published 2\"}");
        merge(&mut m, &mut tools, &next, true);
        let texts: Vec<&str> = m.my_questions.iter().map(|q| q.text.as_str()).collect();
        assert_eq!(texts, ["Published, edited", "Made in the browser", "Published two"]);
        assert!(m.curriculum.contains("Published 2"));
    }

    #[test]
    fn the_live_bank_follows_the_editors_including_what_they_take_back() {
        let mut m = MathsUi::default();
        m.my_questions.push(serde_json::from_value(question("browser-1", "Mine")).unwrap());
        let v1 = json!({"myQuestions": [question("staff-1", "One"), question("staff-2", "Two")], "removedQuestions": ["alg-1"]});
        let s1 = apply_live(&mut m, &v1, &LiveState::default());
        assert_eq!(m.my_questions.len(), 3);
        assert_eq!(m.removed_questions, ["alg-1"]);
        // An editor deletes staff-2, edits staff-1 and puts alg-1 back.
        let v2 = json!({"myQuestions": [question("staff-1", "One, edited")], "removedQuestions": []});
        let s2 = apply_live(&mut m, &v2, &s1);
        let texts: Vec<&str> = m.my_questions.iter().map(|q| q.text.as_str()).collect();
        assert_eq!(texts, ["Mine", "One, edited"]);
        assert!(m.removed_questions.is_empty());
        assert_eq!(s2.questions, ["staff-1"]);
    }

    #[test]
    fn nothing_published_changes_nothing() {
        let (mut m, mut tools) = (MathsUi::default(), MathsToolsSettings::default());
        merge(&mut m, &mut tools, &json!({}), true);
        assert_eq!(m, MathsUi::default());
        assert_eq!(tools, MathsToolsSettings::default());
    }

    #[test]
    fn fingerprints_tell_versions_apart() {
        assert_eq!(fingerprint("a"), fingerprint("a"));
        assert_ne!(fingerprint("a"), fingerprint("b"));
    }
}
