//! The browser shell: web `Services`, drag-and-drop, and the eframe web runner.

use deckcraft_engine::Session;
use deckcraft_ui_egui::{Inbox, InboxEvent, InboxFile, Services, SlideApp};
use wasm_bindgen::JsCast as _;

pub(crate) mod settings;

const PRESENTATION_EXTS: &[&str] = &["deckcraft", "pptx", "potx", "ppsx", "txt", "md"];
const PICTURE_EXTS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "bmp", "tif", "tiff"];
const AUDIO_EXTS: &[&str] = &["wav", "mp3", "m4a", "aac", "flac", "ogg", "opus", "aif", "aiff"];
const VIDEO_EXTS: &[&str] = &["mp4", "m4v", "mov", "webm"];
const CANVAS_ID: &str = "deckcraft_canvas";
const LOADING_ID: &str = "deckcraft_loading";

/// Start the browser shell: install logging, locate the canvas, launch the eframe runner,
/// build the sample or blank deck, and replace the loading element when launch finishes.
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
                    deckcraft_ui_egui::install_ime_pass_hooks(&cc.egui_ctx);
                    if let Some(token) = crate::presenter::token() {
                        let presenter = crate::presenter::Popup::new(token, cc.egui_ctx.clone()).map_err(std::io::Error::other)?;
                        return Ok(Box::new(presenter) as Box<dyn eframe::App>);
                    }
                    let inbox: Inbox = Inbox::default();
                    let mut app = SlideApp::new(Session::new(), services(inbox.clone(), cc.egui_ctx.clone()));
                    let mut settings = settings::Settings::load(&mut app);
                    crate::school::apply(&mut app);
                    settings.save_now(&app);
                    let exam_ready = std::rc::Rc::new(std::cell::Cell::new(false));
                    if crate::school::exam_library() {
                        load_exam_library(exam_ready.clone(), cc.egui_ctx.clone());
                    }
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
                    let smoke = q.contains("smoke").then(crate::smoke::Smoke::default);
                    Ok(Box::new(WebShell { app, inbox, settings, exam_ready, smoke }))
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
    /// Saved to local storage as they change.
    settings: settings::Settings,
    /// The published exam question library has arrived: point the Question Library at it.
    exam_ready: std::rc::Rc<std::cell::Cell<bool>>,
    /// `?smoke`: the start-up check a headless browser runs (`smoke`).
    smoke: Option<crate::smoke::Smoke>,
}

impl eframe::App for WebShell {
    /// Delegate one eframe logic pass to the wrapped application.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.exam_ready.take() {
            self.app.ui.maths.exam_library_dir = crate::school::EXAM_DIR.to_string();
        }
        fetch_wanted(ctx);
        if let Some(s) = self.smoke.as_mut() {
            s.tick(&mut self.app, ctx);
        }
        self.app.logic(ctx);
        if self.app.show.is_none()
            && let Some(host) = self.app.services.presenter.as_mut()
        {
            host.close();
        }
        self.settings.save(&self.app, ctx);
    }

    /// Consume browser drops with fresh input, capture the identity before the asynchronous read,
    /// and deliver the id, name, and bytes through the inbox.
    fn raw_input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        // Browsers cannot read dropped files synchronously. Consume this pass's drops here, with
        // fresh input, and capture the document/slide before the asynchronous read finishes.
        for f in std::mem::take(&mut raw.dropped_files) {
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
        self.app.raw_input_hook(ctx, raw);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.app.ui(ui);
    }
}

/// Web services: no file paths, so `pick_open`/`pick_save`/`read` stay `None` and the UI falls
/// back to `open_async` (browser picker → inbox) and `download`.
fn services(inbox: Inbox, ctx: egui::Context) -> Services {
    let open_inbox = inbox.clone();
    let presenter = match crate::presenter::BrowserHost::new(ctx.clone()) {
        Ok(host) => Some(Box::new(host) as Box<dyn deckcraft_ui_egui::presenter::Host>),
        Err(error) => {
            log::error!("Presenter window: {error}");
            None
        }
    };
    Services {
        presenter,
        open_async: Some(Box::new(move |purpose: &str, request: u64| {
            let inbox = open_inbox.clone();
            let ctx = ctx.clone();
            let dialog = match purpose {
                "picture" => rfd::AsyncFileDialog::new().add_filter("Pictures", PICTURE_EXTS),
                "audio" => rfd::AsyncFileDialog::new().add_filter("Audio", AUDIO_EXTS),
                "video" => rfd::AsyncFileDialog::new().add_filter("Video", VIDEO_EXTS),
                "curriculum" => rfd::AsyncFileDialog::new().add_filter("Curriculum file", &["json"]),
                "questions" => rfd::AsyncFileDialog::new().add_filter("Question file", &["json", "txt"]),
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

/// The exam question library beside a private site: its questions now, each picture when it is
/// first shown (they are too many to download at once). Only counts are logged, never contents.
fn load_exam_library(ready: std::rc::Rc<std::cell::Cell<bool>>, ctx: egui::Context) {
    use deckcraft_engine::web_files;
    wasm_bindgen_futures::spawn_local(async move {
        let dir = crate::school::EXAM_DIR;
        let items = format!("{dir}/items/exam/library.json");
        let crops = format!("{dir}/derived/exam_crops");
        let names =
            fetch_bytes(&format!("{dir}/crops.json")).await.and_then(|b| serde_json::from_slice::<Vec<String>>(&b).map_err(|e| e.to_string()));
        match (fetch_bytes(&items).await, names) {
            (Ok(bytes), Ok(names)) => {
                log::info!("exam question library: {} pictures", names.len());
                web_files::provide(&items, bytes);
                web_files::list(&crops, names);
                web_files::fetch_on_demand(&crops);
                ready.set(true);
                ctx.request_repaint();
            }
            (Err(e), _) | (_, Err(e)) => log::error!("the exam question library could not be loaded: {e}"),
        }
    });
}

/// Fetch the files something asked the browser's file store for (`deckcraft_engine::web_files`).
fn fetch_wanted(ctx: &egui::Context) {
    use deckcraft_engine::web_files;
    for path in web_files::take_wanted() {
        let ctx = ctx.clone();
        wasm_bindgen_futures::spawn_local(async move {
            match fetch_bytes(&path).await {
                Ok(bytes) => web_files::provide(&path, bytes),
                Err(e) => {
                    log::warn!("{e}");
                    web_files::failed(&path);
                }
            }
            ctx.request_repaint();
        });
    }
}

/// The bytes at `url`, relative to the page (so a private site's password goes with the request).
async fn fetch_bytes(url: &str) -> Result<Vec<u8>, String> {
    use wasm_bindgen_futures::JsFuture;
    let js = |e: wasm_bindgen::JsValue| format!("{url}: {e:?}");
    let window = web_sys::window().ok_or("no window")?;
    let resp: web_sys::Response =
        JsFuture::from(window.fetch_with_str(url)).await.map_err(js)?.dyn_into().map_err(|_| format!("{url}: not a response"))?;
    if !resp.ok() {
        return Err(format!("{url}: HTTP {}", resp.status()));
    }
    let buf = JsFuture::from(resp.array_buffer().map_err(js)?).await.map_err(js)?;
    Ok(js_sys::Uint8Array::new(&buf).to_vec())
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
