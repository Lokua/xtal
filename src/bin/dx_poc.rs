use std::{sync::mpsc, thread, time::Duration};

use serde_json::json;
use tao::{
    dpi::{LogicalPosition, LogicalSize},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use wry::WebViewBuilder;

use lattice::framework::prelude::*;

#[derive(Debug)]
enum WebViewEvent {
    SelectInputPort(String),
}

enum WebViewUpdate {
    NewData(serde_json::Value),
}

fn main() -> wry::Result<()> {
    init_logger();

    let event_loop = EventLoop::new();

    // For receiving events from a webview
    let (tx, rx): (mpsc::Sender<WebViewEvent>, mpsc::Receiver<WebViewEvent>) =
        mpsc::channel();

    // Simulate events from our app that we then send into the webview
    let (update_tx, update_rx): (
        mpsc::Sender<WebViewUpdate>,
        mpsc::Receiver<WebViewUpdate>,
    ) = mpsc::channel();

    let window = WindowBuilder::new()
        .with_title("GUI")
        .with_inner_size(LogicalSize::new(400, 700))
        .with_position(LogicalPosition::new(700, 0))
        .build(&event_loop)
        .unwrap();

    let input_ports = midi::list_ports(midi::InputsOrOutputs::Inputs).unwrap();
    let lattice_data = json!({
        "inputPorts": input_ports
            .iter()
            .map(|(_, port)| port)
            .cloned()
            .collect::<Vec<String>>(),
    });

    let web_view = WebViewBuilder::new()
        .with_url("http://localhost:3000/")
        .with_devtools(true)
        .with_initialization_script(&format!(
            "window.latticeData = {}",
            lattice_data
        ))
        .with_ipc_handler(move |message| {
            trace!("raw message: {:?}", message);
            tx.send(WebViewEvent::SelectInputPort(message.body().to_string()))
                .unwrap()
        })
        .build(&window)?;

    let update_tx_clone = update_tx.clone();

    thread::spawn(move || loop {
        let data = json!({
            "event": "lattice::test",
            "data": {
                "timestamp": chrono::Local::now().to_rfc3339(),
                "value": nannou::rand::random::<f64>()
            }
        });

        debug!("generating...");
        if let Err(e) = update_tx_clone.send(WebViewUpdate::NewData(data)) {
            error!("Failed to send update: {:?}", e);
            break;
        }

        thread::sleep(Duration::from_millis(1000));
    });

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        while let Ok(event) = rx.try_recv() {
            match event {
                WebViewEvent::SelectInputPort(port) => {
                    debug!("port: {:?}", port)
                }
            }
        }

        while let Ok(update) = update_rx.try_recv() {
            match update {
                WebViewUpdate::NewData(data) => {
                    debug!("sending...");
                    let script = format!("window.postMessage({}, '*');", data);
                    if let Err(e) = web_view.evaluate_script(&script) {
                        error!("Failed to send data to WebView: {:?}", e);
                    }
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
