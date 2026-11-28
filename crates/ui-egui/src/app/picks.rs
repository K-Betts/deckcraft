//! Native file dialogs that never block the UI thread.
//!
//! A blocking save or open panel runs its own event loop. On macOS that nested loop runs blocks
//! winit queued, which re-enter winit's handler while the UI pass that opened the panel is still
//! inside it, and winit panics (an abort, because the panic crosses a C callback). So desktop
//! hosts run the dialog elsewhere ([`Services::pick_async`]), and the work that follows the
//! answer is kept here and run from a later UI pass.

use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Duration;

use crate::SlideApp;
use crate::services::PickKind;

/// What to do with a dialog's answer: the chosen path, or `None` when it was dismissed.
pub(crate) type PickDone = Box<dyn FnOnce(&mut SlideApp, Option<String>)>;

/// A dialog that is open and the work waiting for its answer.
pub(crate) struct PendingPick {
    rx: Receiver<Option<String>>,
    then: PickDone,
}

impl SlideApp {
    /// Whether a native dialog is open (a second one is not started meanwhile).
    pub fn pick_in_flight(&self) -> bool {
        self.pending_pick.is_some()
    }

    /// Whether this host can show a dialog of `kind`.
    pub(crate) fn can_pick(&self, kind: &PickKind) -> bool {
        self.services.pick_async.is_some()
            || match kind {
                PickKind::Open(_) => self.services.pick_open.is_some(),
                PickKind::Save(_) => self.services.pick_save.is_some(),
                PickKind::Folder(_) => self.services.maths.pick_folder.is_some(),
            }
    }

    /// Shows a native dialog and runs `then` with its answer. Without `pick_async` the blocking
    /// picker answers at once (tests, hosts that can block); with it `then` runs from a later UI
    /// pass. A request while a dialog is already open is dropped. Returns `false` when this host
    /// has no such dialog (`then` is not kept).
    pub(crate) fn pick_then(&mut self, kind: PickKind, then: impl FnOnce(&mut SlideApp, Option<String>) + 'static) -> bool {
        if !self.can_pick(&kind) {
            return false;
        }
        if self.pending_pick.is_some() {
            return true;
        }
        if let Some(spawn) = self.services.pick_async.as_mut() {
            let rx = spawn(kind);
            self.pending_pick = Some(PendingPick { rx, then: Box::new(then) });
            return true;
        }
        let answer = match &kind {
            PickKind::Open(p) => self.services.pick_open.as_mut().and_then(|f| f(p)),
            PickKind::Save(n) => self.services.pick_save.as_mut().and_then(|f| f(n)),
            PickKind::Folder(p) => self.services.maths.pick_folder.as_mut().and_then(|f| f(p)),
        };
        then(self, answer);
        true
    }

    /// Delivers a finished dialog's answer. Called at the start of every UI pass.
    pub(crate) fn poll_pick(&mut self, ctx: &egui::Context) {
        let Some(PendingPick { rx, then }) = self.pending_pick.take() else { return };
        match rx.try_recv() {
            Ok(answer) => then(self, answer),
            Err(TryRecvError::Disconnected) => then(self, None),
            Err(TryRecvError::Empty) => {
                self.pending_pick = Some(PendingPick { rx, then });
                ctx.request_repaint_after(Duration::from_millis(100));
            }
        }
    }

    /// The quit dialog's Save with presentations that need a Save As dialog: once each dialog has
    /// answered, ask for the next one, and quit when every presentation is saved. Stops when a
    /// dialog is dismissed.
    pub(crate) fn continue_quit_after_save(&mut self, ctx: &egui::Context) {
        if !self.quit_when_saved || self.pick_in_flight() {
            return;
        }
        if self.save_dismissed {
            self.quit_when_saved = false;
        } else if self.save_all() {
            self.quit_when_saved = false;
            self.quit_confirmed = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else if !self.pick_in_flight() {
            // Failed without a dialog (the error is in the status bar).
            self.quit_when_saved = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    use std::sync::mpsc::{Sender, channel};

    use deckcraft_engine::Session;

    use crate::services::PickKind;
    use crate::{Services, SlideApp};

    /// An app whose dialogs are answered by hand through the returned senders (one per dialog
    /// shown, in order). The blocking pickers panic: nothing may call them when `pick_async` is set.
    fn app_with_async_dialogs() -> (SlideApp, Rc<RefCell<Vec<(PickKind, Sender<Option<String>>)>>>) {
        let shown: Rc<RefCell<Vec<(PickKind, Sender<Option<String>>)>>> = Rc::default();
        let log = Rc::clone(&shown);
        let services = Services {
            pick_open: Some(Box::new(|_| panic!("a blocking open dialog ran inside the UI pass"))),
            pick_save: Some(Box::new(|_| panic!("a blocking save dialog ran inside the UI pass"))),
            pick_async: Some(Box::new(move |kind| {
                let (tx, rx) = channel();
                log.borrow_mut().push((kind, tx));
                rx
            })),
            ..Services::default()
        };
        (SlideApp::new(Session::with_new(), services), shown)
    }

    #[test]
    fn answer_arrives_on_a_later_pass_not_inside_the_request() {
        let (mut app, shown) = app_with_async_dialogs();
        let got: Rc<RefCell<Option<Option<String>>>> = Rc::default();
        let sink = Rc::clone(&got);
        assert!(app.pick_then(PickKind::Open("open".into()), move |_, p| *sink.borrow_mut() = Some(p)));
        assert!(got.borrow().is_none(), "the continuation must not run inside the request");
        assert!(app.pick_in_flight());
        let ctx = egui::Context::default();
        app.poll_pick(&ctx);
        assert!(got.borrow().is_none(), "nothing answered yet");
        shown.borrow()[0].1.send(Some("/a/b.pptx".into())).unwrap();
        app.poll_pick(&ctx);
        assert_eq!(*got.borrow(), Some(Some("/a/b.pptx".to_string())));
        assert!(!app.pick_in_flight());
    }

    #[test]
    fn a_second_dialog_is_not_started_while_one_is_open() {
        let (mut app, shown) = app_with_async_dialogs();
        let ran = Rc::new(Cell::new(0));
        let r = Rc::clone(&ran);
        assert!(app.pick_then(PickKind::Save("x.pptx".into()), move |_, _| r.set(r.get() + 1)));
        let r = Rc::clone(&ran);
        assert!(app.pick_then(PickKind::Save("y.pptx".into()), move |_, _| r.set(r.get() + 10)));
        assert_eq!(shown.borrow().len(), 1);
        shown.borrow()[0].1.send(None).unwrap();
        app.poll_pick(&egui::Context::default());
        assert_eq!(ran.get(), 1);
    }

    #[test]
    fn a_dialog_that_vanishes_counts_as_dismissed() {
        let (mut app, shown) = app_with_async_dialogs();
        let got: Rc<RefCell<Option<Option<String>>>> = Rc::default();
        let sink = Rc::clone(&got);
        app.pick_then(PickKind::Folder("library".into()), move |_, p| *sink.borrow_mut() = Some(p));
        shown.borrow_mut().clear();
        app.poll_pick(&egui::Context::default());
        assert_eq!(*got.borrow(), Some(None));
    }

    #[test]
    fn save_as_writes_the_file_once_the_dialog_answers() {
        let (mut app, shown) = app_with_async_dialogs();
        let dir = std::env::temp_dir().join(format!("deckcraft-picks-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("saved.deckcraft");
        let after: Rc<Cell<Option<bool>>> = Rc::default();
        let a = Rc::clone(&after);
        app.save_as_then("deckcraft", move |_, saved| a.set(Some(saved)));
        assert!(matches!(shown.borrow()[0].0, PickKind::Save(_)));
        assert!(!path.exists() && after.get().is_none(), "nothing is saved before the answer");
        shown.borrow()[0].1.send(Some(path.to_string_lossy().to_string())).unwrap();
        app.poll_pick(&egui::Context::default());
        assert!(path.exists());
        assert_eq!(after.get(), Some(true));
        assert_eq!(app.session.active().and_then(|d| d.path.clone()), Some(path.to_string_lossy().to_string()));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn dismissed_save_as_reports_not_saved() {
        let (mut app, shown) = app_with_async_dialogs();
        let after: Rc<Cell<Option<bool>>> = Rc::default();
        let a = Rc::clone(&after);
        app.save_as_then("pptx", move |_, saved| a.set(Some(saved)));
        shown.borrow()[0].1.send(None).unwrap();
        app.poll_pick(&egui::Context::default());
        assert_eq!(after.get(), Some(false));
        assert!(app.save_dismissed);
    }

    #[test]
    fn hosts_without_async_dialogs_answer_at_once() {
        let services = Services { pick_open: Some(Box::new(|p| Some(format!("/f/{p}")))), ..Services::default() };
        let mut app = SlideApp::new(Session::with_new(), services);
        let got: Rc<RefCell<Option<Option<String>>>> = Rc::default();
        let sink = Rc::clone(&got);
        assert!(app.pick_then(PickKind::Open("picture".into()), move |_, p| *sink.borrow_mut() = Some(p)));
        assert_eq!(*got.borrow(), Some(Some("/f/picture".to_string())));
        assert!(!app.pick_then(PickKind::Save("n".into()), |_, _| panic!("no save dialog here")));
    }
}
