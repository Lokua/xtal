use std::error::Error;

use ipc_channel::ipc::{self, IpcSender};
use rfd::FileDialog;
use tao::dpi::{self, LogicalPosition, LogicalSize, PixelUnit};
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoop};
use tao::window::{WindowBuilder, WindowSizeConstraints};
use wry::WebViewBuilder;
use xtal2::framework::logging::init_logger;
use xtal2::runtime::web_view as wv;

const OPEN_DEVTOOLS: bool = false;
const DEFAULT_UI_PORT: u16 = 3000;

const DEFAULT_WIDTH: i32 = 560;
const DEFAULT_HEIGHT: i32 = 700;
const HEADER_HEIGHT: i32 = 70;
const FOOTER_HEIGHT: i32 = 96 + 27;
const MIN_SETTINGS_HEIGHT: i32 = 400;

fn main() -> Result<(), Box<dyn Error>> {
    init_logger();
    log::info!("Starting xtal2 web_view_process");

    let server_name = std::env::args()
        .nth(1)
        .ok_or("missing IPC bootstrap server name argument")?;

    let (to_parent, receiver) = setup_ipc_connection(server_name)?;
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
        .build(&event_loop)?;

    let ipc_sender = to_parent.clone();
    let web_view_builder =
        WebViewBuilder::new().with_ipc_handler(move |message| {
            let json_string = message.body().to_string();

            let event = serde_json::from_str::<wv::Event>(&json_string)
                .unwrap_or_else(|err| {
                    log::error!(
                        "JSON parse error: {:?}; Problematic JSON: {}",
                        err,
                        json_string
                    );
                    wv::Event::Error(err.to_string())
                });

            match event {
                wv::Event::ChangeDir(kind) => {
                    match FileDialog::new().pick_folder() {
                        Some(dir) => {
                            let _ = ipc_sender.send(wv::Event::ReceiveDir(
                                kind,
                                dir.to_string_lossy().into_owned(),
                            ));
                        }
                        None => {
                            log::info!("{:?} dir selection cancelled", kind);
                        }
                    }
                }
                _ => {
                    let _ = ipc_sender.send(event);
                }
            }
        });

    let ui_port = std::env::var("XTAL_UI_PORT")
        .unwrap_or_else(|_| DEFAULT_UI_PORT.to_string());
    let web_view = web_view_builder
        .with_url(format!("http://localhost:{ui_port}"))
        .build(&window)?;

    if OPEN_DEVTOOLS {
        web_view.open_devtools();
    }

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match receiver.try_recv() {
            Ok(event) => {
                if matches!(event, wv::Event::Quit) {
                    log::info!(
                        "received quit from parent; shutting down web-view process"
                    );
                    *control_flow = ControlFlow::Exit;
                    return;
                }

                let script = format!(
                    "window.postMessage({}, '*');",
                    serde_json::to_string(&event)
                        .unwrap_or_else(|_| "null".to_string())
                );

                if let Err(err) = web_view.evaluate_script(&script) {
                    log::error!("Failed to send data to WebView: {:?}", err);
                }

                match event {
                    wv::Event::LoadSketch {
                        display_name,
                        controls,
                        perf_mode,
                        sketch_width,
                        ..
                    } => {
                        window.set_title(&format!("{} Controls", display_name));
                        window.set_inner_size(LogicalSize::new(
                            DEFAULT_WIDTH,
                            derive_gui_height(&controls)
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
            Err(err) => {
                if !format!("{:?}", err).contains("Empty") {
                    log::error!("Error receiving message: {:?}", err);
                }
            }
        }

        if let Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } = event
        {
            let _ = to_parent.send(wv::Event::Quit);
            *control_flow = ControlFlow::Exit;
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

fn derive_gui_height(controls: &[wv::Control]) -> i32 {
    let unscientific_offset = controls.len() as i32;

    let controls_height: i32 = controls
        .iter()
        .map(|c| match c.kind {
            wv::ControlKind::Separator => 12,
            _ => 24,
        })
        .sum();

    HEADER_HEIGHT + controls_height + FOOTER_HEIGHT + unscientific_offset
}
