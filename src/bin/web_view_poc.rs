use ipc_channel::ipc::{self, IpcSender};
use tao::{
    dpi::{self, LogicalPosition, LogicalSize, PixelUnit},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Theme, WindowBuilder, WindowSizeConstraints},
};
use wry::WebViewBuilder;

use lattice::{
    framework::prelude::*,
    runtime::web_view::{self as wv, SerializableControl},
};

const DEFAULT_WIDTH: i32 = 560;
const DEFAULT_HEIGHT: i32 = 700;
// Eyeballed from devtools. TODO: parse the variables from the CSS file?
const HEADER_HEIGHT: i32 = 70;
const FOOTER_HEIGHT: i32 = 81 + 27;

fn main() -> wry::Result<()> {
    init_logger();
    info!("web_view_poc started");

    let server_name = std::env::args().nth(1).unwrap();
    let (sender, receiver) = setup_ipc_connection(server_name).unwrap();
    let is_light = matches!(dark_light::detect(), dark_light::Mode::Light);

    let event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_title("GUI")
        .with_theme(Some(ternary!(is_light, Theme::Light, Theme::Dark)))
        .with_inner_size(LogicalSize::new(DEFAULT_WIDTH, DEFAULT_HEIGHT))
        // TODO: set x offset based on actual sketch width (but not in
        // performance mode!)
        .with_position(LogicalPosition::new(700, 0))
        .with_inner_size_constraints(WindowSizeConstraints {
            min_width: Some(PixelUnit::Logical(dpi::LogicalUnit(
                DEFAULT_WIDTH as f64,
            ))),
            min_height: Some(PixelUnit::Logical(dpi::LogicalUnit(
                (HEADER_HEIGHT + FOOTER_HEIGHT + 64) as f64,
            ))),
            max_width: Some(PixelUnit::Logical(dpi::LogicalUnit(
                DEFAULT_WIDTH as f64 + 64.0,
            ))),
            max_height: None,
        })
        .build(&event_loop)
        .unwrap();

    let web_view = WebViewBuilder::new()
        .with_url("http://localhost:3000/")
        .with_devtools(true)
        .with_ipc_handler(move |message| {
            trace!("ipc_handler message: {:?};", message);
            let json_string = message.body().to_string();

            let web_view_event =
                match serde_json::from_str::<wv::Event>(&json_string) {
                    Ok(event) => event,
                    Err(e) => {
                        error!(
                            "JSON parse error: {:?}; Problematic JSON: {}",
                            e, json_string
                        );
                        wv::Event::Error(format!("{}", e))
                    }
                };

            sender.send(web_view_event).unwrap();
        })
        .build(&window)?;

    web_view.open_devtools();

    trace!("Child: Starting event loop");
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match receiver.try_recv() {
            Ok(event) => {
                trace!("Child: Received parent event: {:?}", event);

                let script = format!(
                    "window.postMessage({}, '*');",
                    serde_json::to_string(&event).unwrap()
                );
                if let Err(e) = web_view.evaluate_script(&script) {
                    error!("Failed to send data to WebView: {:?}", e);
                }

                // allowing as likely to add more matches in the future
                #[allow(clippy::single_match)]
                match event {
                    wv::Event::LoadSketch {
                        display_name,
                        controls,
                        ..
                    } => {
                        debug!("Received LoadSketch. Setting title and height");
                        window.set_title(&format!("{} Controls", display_name));
                        window.set_inner_size(LogicalSize::new(
                            DEFAULT_WIDTH,
                            derive_gui_height(controls),
                        ));
                    }
                    _ => {}
                }
            }
            Err(e) => {
                if !format!("{:?}", e).contains("Empty") {
                    error!("Child: Error receiving message: {:?}", e);
                }
            }
        }

        if let Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } = event
        {
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

fn derive_gui_height(controls: Vec<SerializableControl>) -> i32 {
    let controls_height: i32 = controls
        .iter()
        .map(|c| match c {
            SerializableControl::DynamicSeparator { .. }
            | SerializableControl::Separator {} => 9,
            _ => 24,
        })
        .sum();
    let non_scientific_offset = controls.len() as i32;

    let h =
        HEADER_HEIGHT + controls_height + FOOTER_HEIGHT + non_scientific_offset;

    debug!("Derived GUI height: {}", h);

    h
}
