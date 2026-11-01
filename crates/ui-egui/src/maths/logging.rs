//! Deck logging, for reports of trouble with several presentations open: slow frames and slow
//! AutoRecover saves, as counts and timings only (never titles or paths).

use serde_json::{Value, json};

use crate::{SlideApp, now_ms};

/// A frame slower than this (ms) is logged.
pub(crate) const SLOW_FRAME_MS: f64 = 100.0;

impl SlideApp {
    /// Record how long this pass took since `t0`, and log it when it was slow.
    pub(crate) fn note_frame_time(&mut self, t0: f64) {
        let now = now_ms();
        self.perf.frame_ms = now - t0;
        if self.perf.frame_ms > SLOW_FRAME_MS && now - self.perf.slow_logged_ms >= 1000.0 {
            self.perf.slow_logged_ms = now;
            log::warn!(
                "slow frame: {:.0} ms ({:.0} ms drawing the slide), {} presentations open",
                self.perf.frame_ms,
                self.perf.render_ms,
                self.session.documents().len()
            );
        }
    }

    /// Save AutoRecover information, logging how long it took. This runs on the UI thread, so the
    /// window is frozen for as long as it takes.
    pub(crate) fn save_recovery_logged(&mut self) -> Result<Value, deckcraft_engine::EngineError> {
        let t0 = now_ms();
        let saved = self.session.execute("file.recovery.save", &json!({}));
        let ms = now_ms() - t0;
        if ms > SLOW_FRAME_MS {
            log::warn!("AutoRecover froze the window for {ms:.0} ms ({} presentations open)", self.session.documents().len());
        } else {
            log::info!("AutoRecover took {ms:.0} ms");
        }
        saved
    }
}
