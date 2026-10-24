//! Updating DeckCraft from inside the app.
//!
//! The network and the file swap live in the host (`deckcraft-update`, injected through
//! `Services::updates`); this file shows the state and runs commands. Nothing contacts GitHub
//! until the teacher turns on *Check for updates* in Preferences or presses *Check now*, and
//! nothing is installed until they press *Update now*.

use egui::{Color32, CornerRadius, Rect, RichText, Sense, Ui, pos2, vec2};
use serde_json::{Value, json};

use crate::SlideApp;
use crate::theme::{self, Tokens};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateState {
    Idle,
    Checking,
    UpToDate,
    Available {
        label: String,
        sha: String,
        notes: String,
    },
    Installing,
    /// The new app is in place; restarting uses it.
    Installed,
    Failed(String),
}

/// What the host provides. Every method returns at once.
pub trait UpdateBackend {
    fn state(&self) -> UpdateState;
    /// The running app's version, as far as it knows (a commit id, or nothing for a dev build).
    fn current(&self) -> Option<String>;
    fn check(&mut self);
    fn install(&mut self);
    /// Start the new version when this one quits. The caller then quits.
    fn restart(&mut self) -> Result<(), String>;
    /// Take what finished; true if the state changed.
    fn poll(&mut self) -> bool;
}

impl SlideApp {
    pub fn update_state(&self) -> Option<UpdateState> {
        self.services.updates.as_ref().map(|u| u.state())
    }

    /// Every frame: collect finished work, and check once at start-up if the teacher asked for that.
    pub fn poll_updates(&mut self) {
        let auto = self.ui.check_for_updates && !self.update_started;
        let Some(u) = self.services.updates.as_mut() else { return };
        u.poll();
        if auto {
            self.update_started = true;
            u.check();
        }
    }

    pub(crate) fn run_update(&mut self, id: &str) -> Result<Value, String> {
        let Some(u) = self.services.updates.as_mut() else { return Err("Updates are not available in this build".into()) };
        match id {
            "update.check" => u.check(),
            "update.install" => {
                if !matches!(u.state(), UpdateState::Available { .. }) {
                    return Err("there is no update to install; check first".into());
                }
                u.install();
            }
            "update.restart" => {
                if self.session.documents().iter().any(|d| d.is_dirty()) {
                    return Err("save your work first: restarting closes DeckCraft".into());
                }
                u.restart()?;
                self.quit_confirmed = true;
                self.pending_quit = true;
            }
            _ => return Err(format!("unknown update command {id}")),
        }
        Ok(Value::Null)
    }
}

/// The “Update DeckCraft” dialog body. True to close.
pub fn dialog(app: &mut SlideApp, ui: &mut Ui) -> bool {
    let t = Tokens::get(ui.ctx());
    ui.set_min_width(420.0);
    let state = app.update_state().unwrap_or(UpdateState::Idle);
    let current = app.services.updates.as_ref().and_then(|u| u.current()).map(|s| s.chars().take(7).collect::<String>());
    let mut close = false;
    match state {
        UpdateState::Idle => {
            ui.label("Check GitHub for a newer version of DeckCraft? This sends no personal information.");
            ui.horizontal(|ui| {
                if ui.button("Check now").clicked() {
                    let _ = app.run("update.check", json!({}));
                }
                close = ui.button("Close").clicked();
            });
        }
        UpdateState::Checking => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Checking for a newer version…");
            });
        }
        UpdateState::UpToDate => {
            ui.label(format!("You have the latest version{}.", current.map(|c| format!(" ({c})")).unwrap_or_default()));
            ui.horizontal(|ui| {
                if ui.button("Check again").clicked() {
                    let _ = app.run("update.check", json!({}));
                }
                close = ui.button("Close").clicked();
            });
        }
        UpdateState::Available { label, sha, notes } => {
            ui.label(RichText::new(format!("A new version of {} is ready.", deckcraft_engine::brand::APP_NAME)).font(theme::bold(15.0)));
            ui.label(RichText::new(format!("You have: {}    New: {} ({label})", current.unwrap_or_else(|| "unknown".into()), sha)).color(t.text_dim));
            if !notes.trim().is_empty() {
                egui::ScrollArea::vertical().max_height(120.0).show(ui, |ui| {
                    ui.label(notes.trim());
                });
            }
            ui.label(RichText::new("Your open presentations are not touched. You choose when to restart.").size(12.0).color(t.text_dim));
            ui.horizontal(|ui| {
                if ui.add(egui::Button::new(RichText::new("Update now").color(t.accent_text)).fill(t.accent).min_size(vec2(110.0, 30.0))).clicked() {
                    let _ = app.run("update.install", json!({}));
                }
                close = ui.button("Later").clicked();
            });
        }
        UpdateState::Installing => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Downloading and installing… (a few seconds)");
            });
        }
        UpdateState::Installed => {
            ui.label(RichText::new("Updated.").font(theme::bold(15.0)));
            ui.label("Restart DeckCraft to use the new version.");
            let unsaved = app.session.documents().iter().any(|d| d.is_dirty());
            if unsaved {
                ui.colored_label(Color32::from_rgb(200, 60, 60), "You have unsaved changes: save them first, then restart.");
            }
            ui.horizontal(|ui| {
                if ui.add_enabled(!unsaved, egui::Button::new("Restart now").min_size(vec2(110.0, 30.0))).clicked() {
                    match app.run("update.restart", json!({})) {
                        Ok(_) => close = true,
                        Err(e) => app.set_status(&e),
                    }
                }
                close |= ui.button("Later").clicked();
            });
        }
        UpdateState::Failed(msg) => {
            ui.colored_label(Color32::from_rgb(200, 60, 60), msg);
            ui.horizontal(|ui| {
                if ui.button("Try again").clicked() {
                    let _ = app.run("update.check", json!({}));
                }
                close = ui.button("Close").clicked();
            });
        }
    }
    close
}

/// A small pill in the status bar while an update is waiting. True if it was clicked.
pub fn status_chip(app: &SlideApp, ui: &mut Ui, rect: Rect) -> bool {
    let Some(label) = (match app.update_state() {
        Some(UpdateState::Available { .. }) => Some("Update available"),
        Some(UpdateState::Installed) => Some("Restart to finish update"),
        _ => None,
    }) else {
        return false;
    };
    let t = Tokens::get(ui.ctx());
    let w = 22.0 + ui.fonts_mut(|f| f.layout_no_wrap(label.into(), theme::font(12.0), t.text).size().x);
    let r = Rect::from_center_size(pos2(rect.center().x - 40.0, rect.center().y), vec2(w, 18.0));
    let resp = ui.interact(r, ui.id().with("update_chip"), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    ui.painter().rect_filled(r, CornerRadius::same(9), if resp.hovered() { t.accent.gamma_multiply(0.85) } else { t.accent });
    ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, label, theme::font(12.0), t.accent_text);
    resp.on_hover_text("A newer version of DeckCraft is ready").clicked()
}

/// The Home screen card. True if Update was chosen (the dialog opens).
pub fn home_card(app: &mut SlideApp, ui: &mut Ui) {
    let Some(UpdateState::Available { sha, .. }) = app.update_state() else { return };
    let t = Tokens::get(ui.ctx());
    ui.add_space(12.0);
    egui::Frame::new().fill(t.selected).corner_radius(theme::radius::SURFACE).inner_margin(egui::Margin::symmetric(18, 14)).show(ui, |ui| {
        ui.add(egui::Label::new(format!("A new version of DeckCraft is available ({sha}).")).wrap());
        let main = egui::Button::new(RichText::new("See what's new").color(t.accent_text).font(theme::bold(13.0)))
            .fill(t.accent)
            .corner_radius(theme::radius::CONTROL)
            .min_size(vec2(0.0, 30.0));
        if ui.add(main).clicked() {
            app.dialog = Some(crate::dialogs::Dialog::new("update"));
        }
    });
}
