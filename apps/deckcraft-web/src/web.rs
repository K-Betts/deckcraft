//! The browser shell: web `Services`, drag-and-drop, and the eframe web runner.

use deckcraft_engine::Session;
use deckcraft_ui_egui::{Inbox, InboxEvent, InboxFile, Services, SlideApp};
use wasm_bindgen::JsCast as _;

const PRESENTATION_EXTS: &[&str] = &["deckcraft", "pptx", "potx", "ppsx", "txt", "md"];
const PICTURE_EXTS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "bmp", "tif", "tiff"];
const AUDIO_EXTS: &[&str] = &["wav", "mp3", "m4a", "aac", "flac", "ogg", "opus", "aif", "aiff"];
const VIDEO_EXTS: &[&str] = &["mp4", "m4v", "mov", "webm"];
const CANVAS_ID: &str = "deckcraft_canvas";
const LOADING_ID: &str = "deckcraft_loading";

pub fn start() {
    eframe::WebLogger::init(log::LevelFilter::Info).ok();
    wasm_bindgen_futures::spawn_local(async {
        let Some(document) = web_sys::window().and_then(|w| w.document()) else {
            log::error!("no document");
            return;
        };
        let Some(canvas) = document.get_element_by_id(CANVAS_ID).and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok()) else {
            log::error!("missing <canvas id=\"{CANVAS_ID}\">");
            return;
        };
        let mut options = eframe::WebOptions::default();
        if query().contains("webgl")
            && let eframe::egui_wgpu::WgpuSetup::CreateNew(create) = &mut options.wgpu_options.wgpu_setup
        {
            create.instance_descriptor.backends = eframe::wgpu::Backends::GL;
        }
        let result = eframe::WebRunner::new()
            .start(
                canvas,
                options,
                Box::new(move |cc| {
                    if let Some(rs) = &cc.wgpu_render_state {
                        log::info!("deckcraft-web {}: wgpu backend {:?}", env!("CARGO_PKG_VERSION"), rs.adapter.get_info().backend);
                    }
                    let inbox: Inbox = Inbox::default();
                    let mut app = SlideApp::new(Session::new(), services(inbox.clone(), cc.egui_ctx.clone()));
                    let saved = load_settings(&mut app);
                    let q = query();
                    // A teacher opens the app to make or open a deck, so it starts on the Home
                    // screen with an empty presentation behind it; `?sample` loads the demo deck.
                    if q.contains("sample") {
                        if let Err(e) = deckcraft_engine::sample::open_sample(&mut app.session) {
                            log::error!("sample deck: {e}");
                        }
                    } else if let Err(e) = app.run("file.new", serde_json::json!({})) {
                        log::error!("new presentation: {e}");
                    } else if !q.contains("show") {
                        let _ = app.run("app.home", serde_json::json!({"on": true}));
                    }
                    if q.contains("show") {
                        app.start_show(0, false);
                    }
                    Ok(Box::new(WebShell { app, inbox, saved, last_save: 0.0 }))
                }),
            )
            .await;
        if let Some(el) = document.get_element_by_id(LOADING_ID) {
            match result {
                Ok(()) => el.remove(),
                Err(e) => el.set_inner_html(&format!("<p>DeckCraft failed to start: {e:?}</p><p>A browser with WebGPU or WebGL2 is required.</p>")),
            }
        }
    });
}

fn query() -> String {
    web_sys::window().and_then(|w| w.location().search().ok()).unwrap_or_default()
}

/// Wraps the app to read dropped files asynchronously (browsers can't read them synchronously)
/// and feed them through the inbox.
struct WebShell {
    app: SlideApp,
    inbox: Inbox,
    /// The settings text as last written to local storage, to write only when something changed.
    saved: [String; 3],
    last_save: f64,
}

const SETTINGS_KEYS: [&str; 3] = ["deckcraft.ui", "deckcraft.prefs", "deckcraft.mathsTools"];
/// The most often (seconds) the settings are written to local storage.
const SAVE_EVERY: f64 = 1.0;

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

/// Read the saved settings into `app` (a missing, blocked or damaged store leaves the defaults:
/// private windows and locked-down school browsers can refuse local storage).
fn load_settings(app: &mut SlideApp) -> [String; 3] {
    use deckcraft_ui_egui::settings::text;
    let Some(s) = storage() else { return Default::default() };
    let get = |i: usize| s.get_item(SETTINGS_KEYS[i]).ok().flatten().unwrap_or_default();
    let saved = [get(0), get(1), get(2)];
    if let Some(ui) = text::parse_ui(&saved[0]) {
        app.ui = ui;
    }
    if let Some(prefs) = text::parse_prefs(&saved[1]) {
        app.session.prefs = prefs;
    }
    if let Some(m) = text::parse_maths_tools(&saved[2]) {
        app.maths_tools = m;
    }
    saved
}

impl WebShell {
    /// Keep the three settings in local storage up to date. They are compared every frame (three
    /// small JSON strings), and written when they differ, at most once a `SAVE_EVERY` seconds. A
    /// change held back by that limit asks for one more frame when the wait is over, because the
    /// app only runs a frame when something happens. A refused write is ignored; nothing depends on it.
    fn save_settings(&mut self, ctx: &egui::Context) {
        use deckcraft_ui_egui::settings::text;
        let Some(s) = storage() else { return };
        let now = ctx.input(|i| i.time);
        let now_text = [text::ui(&self.app.ui), text::prefs(&self.app.session.prefs), text::maths_tools(&self.app.maths_tools)];
        if now_text == self.saved {
            return;
        }
        if now - self.last_save < SAVE_EVERY {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(SAVE_EVERY - (now - self.last_save) + 0.05));
            return;
        }
        self.last_save = now;
        for i in 0..3 {
            if now_text[i] != self.saved[i] && s.set_item(SETTINGS_KEYS[i], &now_text[i]).is_ok() {
                self.saved[i] = now_text[i].clone();
            }
        }
    }
}

impl eframe::App for WebShell {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let dropped = ctx.input_mut(|i| std::mem::take(&mut i.raw.dropped_files));
        for f in dropped {
            // Capture the document and slide now. The bytes arrive later, after the user may
            // have switched slides.
            let id = self.app.begin_external_file("drop");
            let inbox = self.inbox.clone();
            let ctx = ctx.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let name = f.path().file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "dropped".into());
                match f.bytes_async().await {
                    Ok(bytes) => {
                        push_inbox(&inbox, InboxEvent::File(InboxFile { id, name, bytes }));
                        ctx.request_repaint();
                    }
                    Err(e) => {
                        log::error!("couldn't read dropped file {name}: {e}");
                        push_inbox(&inbox, InboxEvent::Cancelled { id });
                        ctx.request_repaint();
                    }
                }
            });
        }
        self.app.logic(ctx);
        self.save_settings(ctx);
    }

    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw: &mut egui::RawInput) {
        self.app.raw_input_hook(raw);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.app.ui(ui);
    }
}

/// Web services: no file paths, so `pick_open`/`pick_save`/`read` stay `None` and the UI falls
/// back to `open_async` (browser picker → inbox) and `download`.
fn services(inbox: Inbox, ctx: egui::Context) -> Services {
    let open_inbox = inbox.clone();
    Services {
        open_async: Some(Box::new(move |purpose: &str, request: u64| {
            let inbox = open_inbox.clone();
            let ctx = ctx.clone();
            let dialog = match purpose {
                "picture" => rfd::AsyncFileDialog::new().add_filter("Pictures", PICTURE_EXTS),
                "audio" => rfd::AsyncFileDialog::new().add_filter("Audio", AUDIO_EXTS),
                "video" => rfd::AsyncFileDialog::new().add_filter("Video", VIDEO_EXTS),
                _ => rfd::AsyncFileDialog::new().add_filter("Presentations", PRESENTATION_EXTS),
            };
            wasm_bindgen_futures::spawn_local(async move {
                let Some(file) = dialog.pick_file().await else {
                    push_inbox(&inbox, InboxEvent::Cancelled { id: request });
                    ctx.request_repaint();
                    return;
                };
                let bytes = file.read().await;
                push_inbox(&inbox, InboxEvent::File(InboxFile { id: request, name: file.file_name(), bytes }));
                ctx.request_repaint();
            });
        })),
        write: Some(Box::new(|name: &str, bytes: &[u8]| download(name, bytes))),
        download: Some(Box::new(|name: &str, bytes: &[u8]| {
            if let Err(e) = download(name, bytes) {
                log::error!("download of {name} failed: {e}");
            }
        })),
        inbox: Some(inbox),
        ..Default::default()
    }
}

fn push_inbox(inbox: &Inbox, event: InboxEvent) {
    inbox.lock().unwrap_or_else(|e| e.into_inner()).push(event);
}

/// Trigger a browser download of `bytes` named after the last component of `path`.
fn download(path: &str, bytes: &[u8]) -> Result<(), String> {
    let js = |e: wasm_bindgen::JsValue| format!("{e:?}");
    let name = std::path::Path::new(path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "deckcraft".into());
    let window = web_sys::window().ok_or("no window")?;
    let document = window.document().ok_or("no document")?;
    let parts = js_sys::Array::of1(&js_sys::Uint8Array::from(bytes));
    let opts = web_sys::BlobPropertyBag::new();
    opts.set_type(mime_for(&name));
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &opts).map_err(js)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(js)?;
    let a: web_sys::HtmlAnchorElement = document.create_element("a").map_err(js)?.dyn_into().map_err(|_| "not an anchor")?;
    a.set_href(&url);
    a.set_download(&name);
    a.style().set_property("display", "none").map_err(js)?;
    let body = document.body().ok_or("no body")?;
    body.append_child(&a).map_err(js)?;
    a.click();
    a.remove();
    // Revoke after the click has been dispatched; the download keeps its own reference.
    let revoke = wasm_bindgen::closure::Closure::once_into_js(move || {
        web_sys::Url::revoke_object_url(&url).ok();
    });
    window.set_timeout_with_callback_and_timeout_and_arguments_0(revoke.unchecked_ref(), 10_000).map_err(js)?;
    Ok(())
}

fn mime_for(name: &str) -> &'static str {
    match name.rsplit('.').next().map(str::to_ascii_lowercase).as_deref() {
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("pdf") => "application/pdf",
        Some("gif") => "image/gif",
        Some("txt") => "text/plain",
        Some("pptx") => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        Some("deckcraft") => "application/zip",
        _ => "application/octet-stream",
    }
}
