//! Files the browser build is given instead of a disk (maths edition).
//!
//! The web app has no folders, so [`crate::io`] reads from this store there. The web shell puts
//! files in it: a private build's own templates at start-up ([`provide`]), and the exam question
//! library, whose pictures are too many to download at once, as a list of names ([`list`]) under
//! a folder fetched one file at a time when something reads it ([`fetch_on_demand`]). A read of
//! such a file that has not arrived yet fails with [`LOADING`] and asks the shell for it
//! ([`take_wanted`]); the shell [`provide`]s it (or marks it [`failed`]) and the screen asks again.
//!
//! It is plain Rust on every target, so it is tested natively; only the wasm `io` functions use it.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, MutexGuard};

/// The error text of a read that is waiting for the file to arrive.
pub const LOADING: &str = "still loading";

#[derive(Default)]
struct Store {
    files: BTreeMap<String, Arc<Vec<u8>>>,
    /// Names of files known to be in a folder before they are fetched.
    listed: BTreeMap<String, BTreeSet<String>>,
    /// Folders whose files are fetched when read.
    remote: Vec<String>,
    /// Asked for and not arrived yet; `wanted` is the part the shell has not started.
    pending: BTreeSet<String>,
    wanted: Vec<String>,
    failed: BTreeSet<String>,
}

static STORE: Mutex<Option<Store>> = Mutex::new(None);

fn with<R>(f: impl FnOnce(&mut Store) -> R) -> R {
    let mut guard: MutexGuard<'_, Option<Store>> = STORE.lock().unwrap_or_else(|e| e.into_inner());
    f(guard.get_or_insert_with(Store::default))
}

fn clean(path: &str) -> String {
    path.trim_end_matches('/').to_string()
}

fn parent(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(dir, _)| dir)
}

/// Put the file `path` in the store (replacing any earlier copy).
pub fn provide(path: &str, bytes: Vec<u8>) {
    let path = clean(path);
    with(|s| {
        s.pending.remove(&path);
        s.failed.remove(&path);
        s.files.insert(path, Arc::new(bytes));
    });
}

/// The files `names` are in the folder `dir`, to be fetched when read.
pub fn list(dir: &str, names: impl IntoIterator<Item = String>) {
    with(|s| s.listed.entry(clean(dir)).or_default().extend(names));
}

/// Files under `dir` that are not in the store are fetched when something reads them.
pub fn fetch_on_demand(dir: &str) {
    let dir = clean(dir);
    with(|s| {
        if !s.remote.contains(&dir) {
            s.remote.push(dir);
        }
    });
}

/// The file `path` could not be fetched: reads of it fail from now on.
pub fn failed(path: &str) {
    let path = clean(path);
    with(|s| {
        s.pending.remove(&path);
        s.failed.insert(path);
    });
}

/// The files asked for since the last call, for the shell to fetch.
pub fn take_wanted() -> Vec<String> {
    with(|s| std::mem::take(&mut s.wanted))
}

/// Whether `path` is on its way (a read of it failed with [`LOADING`]).
pub fn is_loading(path: &str) -> bool {
    with(|s| s.pending.contains(&clean(path)))
}

/// The bytes of `path`; asks for it when it is under a [`fetch_on_demand`] folder.
pub fn read(path: &str) -> Result<Vec<u8>, String> {
    let path = clean(path);
    with(|s| {
        if let Some(bytes) = s.files.get(&path) {
            return Ok(bytes.as_ref().clone());
        }
        if s.pending.contains(&path) {
            return Err(LOADING.into());
        }
        let remote = s.remote.iter().any(|d| path.starts_with(&format!("{d}/")));
        if remote && !s.failed.contains(&path) && !path.split('/').any(|p| p == "..") {
            s.pending.insert(path.clone());
            s.wanted.push(path);
            return Err(LOADING.into());
        }
        Err(format!("{path}: not available in the browser"))
    })
}

/// The files directly in `dir`: those in the store and those [`list`]ed there.
pub fn files_in(dir: &str) -> Vec<String> {
    let dir = clean(dir);
    with(|s| {
        let mut out: BTreeSet<String> = s.files.keys().filter(|p| parent(p) == dir).cloned().collect();
        if let Some(names) = s.listed.get(&dir) {
            out.extend(names.iter().map(|n| format!("{dir}/{n}")));
        }
        out.into_iter().collect()
    })
}

/// Whether anything is in or listed under `dir`.
pub fn has_dir(dir: &str) -> bool {
    !files_in(dir).is_empty()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    // One store for the process: each test uses its own folder.

    #[test]
    fn provided_files_are_read_and_listed_by_folder() {
        provide("/t1/Templates/Lesson.deckcraft", b"deck".to_vec());
        provide("/t1/Templates/sub/x.deckcraft", b"x".to_vec());
        assert_eq!(read("/t1/Templates/Lesson.deckcraft").unwrap(), b"deck");
        assert_eq!(files_in("/t1/Templates/"), ["/t1/Templates/Lesson.deckcraft"]);
        assert!(read("/t1/Templates/missing.deckcraft").unwrap_err().contains("not available"));
        assert!(has_dir("/t1/Templates") && !has_dir("/t1/none"));
    }

    #[test]
    fn a_remote_file_is_asked_for_once_then_read_when_it_arrives() {
        fetch_on_demand("t2/exam");
        list("t2/exam/crops", ["a.png".to_string(), "b.png".to_string()]);
        assert_eq!(files_in("t2/exam/crops"), ["t2/exam/crops/a.png", "t2/exam/crops/b.png"]);
        assert_eq!(read("t2/exam/crops/a.png").unwrap_err(), LOADING);
        assert_eq!(read("t2/exam/crops/a.png").unwrap_err(), LOADING);
        assert!(is_loading("t2/exam/crops/a.png"));
        let wanted: Vec<String> = take_wanted().into_iter().filter(|w| w.starts_with("t2/")).collect();
        assert_eq!(wanted, ["t2/exam/crops/a.png"], "asked for once");
        provide("t2/exam/crops/a.png", vec![1, 2]);
        assert_eq!(read("t2/exam/crops/a.png").unwrap(), [1, 2]);
        assert_eq!(read("t2/exam/crops/b.png").unwrap_err(), LOADING);
        failed("t2/exam/crops/b.png");
        assert!(read("t2/exam/crops/b.png").unwrap_err().contains("not available"), "a failed fetch is not retried");
        assert!(read("t2/exam/../secret").is_err() && !take_wanted().iter().any(|w| w.contains("..")));
        assert!(read("t2/other.png").unwrap_err().contains("not available"), "only remote folders are fetched");
    }
}
