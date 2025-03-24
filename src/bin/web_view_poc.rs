use ipc_channel::ipc::{self, IpcSender};
use tao::{
    dpi::{LogicalPosition, LogicalSize},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Theme, WindowBuilder},
};
use wry::WebViewBuilder;

use lattice::{framework::prelude::*, runtime::web_view as wv};

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
        .with_inner_size(LogicalSize::new(538, 700))
        .with_position(LogicalPosition::new(700, 0))
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
                        error!("JSON parse error: {:?}", e);
                        error!("Problematic JSON: {}", json_string);
                        wv::Event::new("error")
                    }
                };

            sender.send(web_view_event).unwrap();
        })
        .build(&window)?;

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

                #[allow(clippy::single_match)]
                match event.data {
                    Some(wv::Data::LoadSketch { display_name, .. }) => {
                        debug!("Received LoadSketch. Attempting to set title");
                        window.set_title(&format!("{} Controls", display_name));
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
