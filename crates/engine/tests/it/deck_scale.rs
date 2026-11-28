//! How the app copes with several picture-heavy presentations open at once.
//!
//! Teachers report trouble once more than two decks are open. This builds decks in code (never
//! a real deck: no pupil data) and times what the UI thread does on a timer or on open: the
//! AutoRecover save, serialising a deck, and opening one. Ignored by default because it is a
//! measurement, not a check; run it with
//!
//! ```text
//! cargo test -p deckcraft-engine --test it deck_scale -- --ignored --nocapture
//! ```
//!
//! `DECKCRAFT_SCALE_SLIDES` (default 20), `DECKCRAFT_SCALE_PICS` (pictures a slide, default 2)
//! and `DECKCRAFT_SCALE_DECKS` (default 5) size the decks; `DECKCRAFT_SCALE_DIR` says where the
//! files go (default: the system temp folder).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stdout)]

use std::path::{Path, PathBuf};
use std::time::Instant;

use deckcraft_engine::Session;
use deckcraft_engine::cmd::base64_encode;
use serde_json::json;

fn env_n(name: &str, default: usize) -> usize {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

/// A PNG of noise, which does not compress: a stand-in for a photograph.
fn noisy_png(seed: u64) -> Vec<u8> {
    let mut x = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    let img = image::RgbaImage::from_fn(600, 400, |_, _| {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        image::Rgba([x as u8, (x >> 8) as u8, (x >> 16) as u8, 255])
    });
    let mut out = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png).unwrap();
    out
}

fn dir_bytes(dir: &Path) -> u64 {
    std::fs::read_dir(dir).map(|rd| rd.flatten().filter_map(|e| e.metadata().ok()).map(|m| m.len()).sum()).unwrap_or(0)
}

/// Resident memory of this process in MB, from `ps` (0 when `ps` is not there).
fn rss_mb() -> u64 {
    let out = std::process::Command::new("ps").args(["-o", "rss=", "-p", &std::process::id().to_string()]).output();
    out.ok().and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<u64>().ok()).map_or(0, |kb| kb / 1024)
}

fn ms(t: Instant) -> u128 {
    t.elapsed().as_millis()
}

/// One more picture-heavy presentation in the session; it becomes the active one.
fn add_deck(s: &mut Session, seed: u64, slides: usize, pics: usize) {
    s.execute("file.new", &json!({})).unwrap();
    for i in 0..slides {
        s.execute("slide.new", &json!({"layout": "blank"})).unwrap();
        for p in 0..pics {
            let png = noisy_png(seed * 100_000 + (i * pics + p) as u64);
            let rect = [30.0 + 440.0 * p as f64, 40.0, 420.0, 280.0];
            s.execute("insert.picture", &json!({"data": base64_encode(&png), "name": "Photo.png", "rect": rect})).unwrap();
        }
    }
}

#[test]
#[ignore = "a measurement: run it by hand with --ignored --nocapture"]
fn several_big_decks_open_at_once() {
    let (slides, pics, decks) = (env_n("DECKCRAFT_SCALE_SLIDES", 20), env_n("DECKCRAFT_SCALE_PICS", 2), env_n("DECKCRAFT_SCALE_DECKS", 5));
    let root: PathBuf =
        std::env::var_os("DECKCRAFT_SCALE_DIR").map_or_else(std::env::temp_dir, PathBuf::from).join(format!("deck-scale-{}", std::process::id()));
    let recovery = root.join("recovery");
    std::fs::create_dir_all(&recovery).unwrap();

    println!("decks of {slides} slides with {pics} photos each; rss at start {} MB", rss_mb());
    let mut s = Session::new();
    s.recovery_dir = Some(recovery.clone());

    println!("{:>5} {:>10} {:>14} {:>14} {:>12} {:>10}", "decks", "build ms", "recovery #1 ms", "recovery #2 ms", "snapshot MB", "rss MB");
    for n in 1..=decks {
        let t = Instant::now();
        add_deck(&mut s, n as u64, slides, pics);
        let build = ms(t);
        // The first save writes every deck; the second changes nothing, yet every unsaved deck is
        // serialised and written again.
        let t = Instant::now();
        s.execute("file.recovery.save", &json!({})).unwrap();
        let first = ms(t);
        let t = Instant::now();
        s.execute("file.recovery.save", &json!({})).unwrap();
        let second = ms(t);
        println!("{n:>5} {build:>10} {first:>14} {second:>14} {:>12} {:>10}", dir_bytes(&recovery) / (1024 * 1024), rss_mb());
    }

    // One deck written out and opened again, as a teacher opening a second or third deck does.
    let doc = s.documents().last().unwrap().doc.clone();
    let t = Instant::now();
    let bytes = deckcraft_engine::cmd::file::save_bytes(&doc, "deckcraft").unwrap();
    let encode = ms(t);
    let file = root.join("one.deckcraft");
    std::fs::write(&file, &bytes).unwrap();
    let t = Instant::now();
    s.execute("file.open", &json!({"path": file.to_string_lossy()})).unwrap();
    println!("one deck: {} MB, serialise {encode} ms, file.open {} ms, rss {} MB", bytes.len() / (1024 * 1024), ms(t), rss_mb());

    // `DECKCRAFT_SCALE_KEEP=1` leaves the decks in `<dir>/decks` to open in the app.
    if std::env::var_os("DECKCRAFT_SCALE_KEEP").is_some() {
        let keep = file.parent().and_then(Path::parent).unwrap_or(&root).join("decks");
        std::fs::create_dir_all(&keep).unwrap();
        for (i, d) in s.documents().iter().enumerate() {
            let bytes = deckcraft_engine::cmd::file::save_bytes(&d.doc, "deckcraft").unwrap();
            std::fs::write(keep.join(format!("deck-{}.deckcraft", i + 1)), bytes).unwrap();
        }
        println!("kept {} decks in {}", s.documents().len(), keep.display());
    }
    let _ = std::fs::remove_dir_all(&root);
}
