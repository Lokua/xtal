use ipc_channel::ipc::{self, IpcSender};
use rfd::FileDialog;
use rust_embed::Embed;
use std::error::Error;
use tao::dpi::{self, LogicalPosition, LogicalSize, PixelUnit};
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoop};
use tao::window::{WindowBuilder, WindowSizeConstraints};
use wry::WebViewBuilder;
use wry::http::header::CONTENT_TYPE;
use wry::http::{Request, Response};

use super::web_view::{self as wv};
use crate::framework::prelude::*;

const DEFAULT_WIDTH: i32 = 560;
const DEFAULT_HEIGHT: i32 = 700;

// Eyeballed from devtools
const HEADER_HEIGHT: i32 = 70;
const FOOTER_HEIGHT: i32 = 96 + 27;
const MIN_SETTINGS_HEIGHT: i32 = 700;

// #[cfg(feature = "prod")]
#[derive(Embed)]
#[folder = "static"]
struct Asset;

pub fn run() -> Result<(), Box<dyn Error>> {
    init_logger();
    info!("Starting web_view_process");

    let server_name = std::env::args().nth(1).unwrap();
    let (sender, receiver) = setup_ipc_connection(server_name).unwrap();
    let event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_title("Xtal UI")
        .with_inner_size(LogicalSize::new(DEFAULT_WIDTH, DEFAULT_HEIGHT))
        .with_position(LogicalPosition::new(700, 0))
        .with_inner_size_constraints(WindowSizeConstraints {
            min_width: Some(PixelUnit::Logical(dpi::LogicalUnit(
                DEFAULT_WIDTH as f64,
            ))),
            min_height: Some(PixelUnit::Logical(dpi::LogicalUnit(
                MIN_SETTINGS_HEIGHT as f64,
            ))),
            max_width: Some(PixelUnit::Logical(dpi::LogicalUnit(
                DEFAULT_WIDTH as f64 + 64.0,
            ))),
            max_height: None,
        })
        .build(&event_loop)
        .unwrap();

    let web_view_builder = WebViewBuilder::new()
        .with_devtools(true)
        // Events from UI -> Here -> Parent
        .with_ipc_handler(move |message| {
            trace!("ipc_handler message: {:?};", message);
            let json_string = message.body().to_string();

            let event = serde_json::from_str::<wv::Event>(&json_string)
                .unwrap_or_else(|e| {
                    error!(
                        "JSON parse error: {:?}; Problematic JSON: {}",
                        e, json_string
                    );
                    wv::Event::Error(format!("{}", e))
                });

            match event {
                wv::Event::ChangeDir(kind) => {
                    match FileDialog::new().pick_folder() {
                        Some(dir) => {
                            sender
                                .send(wv::Event::ReceiveDir(
                                    kind,
                                    dir.to_string_lossy().into_owned(),
                                ))
                                .unwrap();
                        }
                        None => {
                            info!("{:?} dir selection cancelled", kind);
                        }
                    }
                }
                _ => sender.send(event).unwrap(),
            }
        });

    let web_view = if cfg!(feature = "prod") {
        debug!("Using `prod` protocol handler");
        web_view_builder
            .with_url("app://index.html")
            .with_custom_protocol("app".into(), move |_webview_id, request| {
                match serve(request) {
                    Ok(r) => r.map(Into::into),
                    Err(e) => Response::builder()
                        .header(CONTENT_TYPE, "text/plain")
                        .status(500)
                        .body(e.to_string().as_bytes().to_vec())
                        .unwrap()
                        .map(Into::into),
                }
            })
            .build(&window)?
    } else {
        web_view_builder
            .with_url("http://localhost:3000")
            .build(&window)?
    };

    // web_view.open_devtools();

    trace!("Starting event loop");
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match receiver.try_recv() {
            Ok(event) => {
                trace!("Received parent event: {:?}", event);

                let script = format!(
                    "window.postMessage({}, '*');",
                    serde_json::to_string(&event).unwrap()
                );
                if let Err(e) = web_view.evaluate_script(&script) {
                    error!("Failed to send data to WebView: {:?}", e);
                }

                // Events from Parent -> Here (not for UI)
                match event {
                    wv::Event::LoadSketch {
                        display_name,
                        controls,
                        perf_mode,
                        sketch_width,
                        ..
                    } => {
                        trace!("Received LoadSketch. Setting title and height");
                        window.set_title(&format!("{} Controls", display_name));
                        window.set_inner_size(LogicalSize::new(
                            DEFAULT_WIDTH,
                            derive_gui_height(controls)
                                .max(MIN_SETTINGS_HEIGHT),
                        ));
                        if !perf_mode {
                            window.set_outer_position(LogicalPosition::new(
                                sketch_width,
                                0,
                            ));
                        }
                    }
                    wv::Event::ToggleGuiFocus => {
                        window.set_visible(true);
                        window.set_focus();
                    }
                    _ => {}
                }
            }
            Err(e) => {
                if !format!("{:?}", e).contains("Empty") {
                    error!("Error receiving message: {:?}", e);
                }
            }
        }

        #[allow(clippy::single_match)]
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            _ => {}
        }
    });
}

fn setup_ipc_connection(
    server_name: String,
) -> Result<(wv::Sender, wv::Receiver), ipc_channel::Error> {
    let (to_child, from_parent): (wv::Sender, wv::Receiver) = ipc::channel()?;
    let (to_parent, from_child): (wv::Sender, wv::Receiver) = ipc::channel()?;
    let bootstrap = IpcSender::connect(server_name)?;
    bootstrap.send((to_child, from_child))?;
    Ok((to_parent, from_parent))
}

fn derive_gui_height(controls: Vec<wv::Control>) -> i32 {
    let unscientific_offset = controls.len() as i32;

    let controls_height: i32 = controls
        .iter()
        .map(|c| match c.kind {
            wv::ControlKind::DynamicSeparator | wv::ControlKind::Separator => 9,
            _ => 24,
        })
        .sum();

    let h =
        HEADER_HEIGHT + controls_height + FOOTER_HEIGHT + unscientific_offset;

    trace!("Derived GUI height: {}", h);

    h
}

fn serve(
    request: Request<Vec<u8>>,
) -> Result<Response<Vec<u8>>, Box<dyn std::error::Error>> {
    let uri_path = request.uri().path();
    let path = ternary!(uri_path == "/", "index.html", &uri_path[1..]);

    let asset =
        Asset::get(path).ok_or_else(|| format!("Asset not found: {}", path))?;

    let content = asset.data.into_owned();

    // (can replace with `mime_guess` if needed)
    let mimetype = if path.ends_with(".html") {
        "text/html"
    } else if path.ends_with(".js") {
        "text/javascript"
    } else if path.ends_with(".css") {
        "text/css"
    } else {
        "application/octet-stream"
    };

    Response::builder()
        .header(CONTENT_TYPE, mimetype)
        .body(content)
        .map_err(Into::into)
}
