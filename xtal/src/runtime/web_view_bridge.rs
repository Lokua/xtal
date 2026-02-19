use std::collections::VecDeque;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use ipc_channel::ipc::{IpcOneShotServer, IpcReceiver, IpcSender};
use log::{debug, error, info, trace, warn};

use super::events::{RuntimeCommandSender, RuntimeEvent, RuntimeEventReceiver};
use super::web_view::{self, Event};

type Bootstrap = (IpcSender<Event>, IpcReceiver<Event>);

pub struct WebViewBridge {
    child: Child,
    outbound_handle: Option<JoinHandle<()>>,
    inbound_handle: Option<JoinHandle<()>>,
}

impl WebViewBridge {
    pub fn launch(
        command_tx: RuntimeCommandSender,
        runtime_events: RuntimeEventReceiver,
    ) -> Result<Self, String> {
        let (server, server_name) = IpcOneShotServer::<Bootstrap>::new()
            .map_err(|err| {
                format!("failed to create IPC bootstrap: {}", err)
            })?;

        let mut child = spawn_web_view_process(&server_name)?;

        pipe_child_logs(&mut child);

        let (_bootstrap_rx, (to_child, from_child)): (
            IpcReceiver<Bootstrap>,
            Bootstrap,
        ) = server.accept().map_err(|err| {
            format!("failed to accept web-view bootstrap: {}", err)
        })?;

        info!("web-view process connected");

        let ready = Arc::new(AtomicBool::new(false));
        let queued_events = Arc::new(Mutex::new(VecDeque::<Event>::new()));

        let outbound_handle = {
            let to_child = to_child.clone();
            let ready = ready.clone();
            let queued_events = queued_events.clone();

            thread::spawn(move || {
                while let Ok(runtime_event) = runtime_events.recv() {
                    match runtime_event {
                        RuntimeEvent::WebView(event) => {
                            let event = *event;
                            if ready.load(Ordering::Acquire) {
                                if let Err(err) = to_child.send(event) {
                                    warn!(
                                        "failed to send event to web-view process: {}",
                                        err
                                    );
                                    break;
                                }
                            } else if let Ok(mut queue) = queued_events.lock() {
                                queue.push_back(event);
                            }
                        }
                        RuntimeEvent::Stopped => {
                            let _ = to_child.send(Event::Quit);
                            break;
                        }
                        RuntimeEvent::FrameSkipped
                        | RuntimeEvent::SketchSwitched(_) => {}
                        _ => {}
                    }
                }
            })
        };

        let inbound_handle = {
            let command_tx = command_tx.clone();
            let to_child = to_child.clone();
            let ready = ready.clone();
            let queued_events = queued_events.clone();

            thread::spawn(move || {
                while let Ok(event) = from_child.recv() {
                    trace!("received web-view event: {:?}", event);

                    if matches!(event, Event::Ready) {
                        ready.store(true, Ordering::Release);
                        flush_queued_events(&to_child, &queued_events);
                        continue;
                    }

                    if let Some(command) =
                        web_view::map_event_to_runtime_event(&event)
                    {
                        if let Err(err) = command_tx.send(command) {
                            warn!(
                                "failed to dispatch runtime command from web-view: {}",
                                err
                            );
                            break;
                        }
                    }
                }
            })
        };

        Ok(Self {
            child,
            outbound_handle: Some(outbound_handle),
            inbound_handle: Some(inbound_handle),
        })
    }
}

impl Drop for WebViewBridge {
    fn drop(&mut self) {
        debug!("shutting down web-view bridge");

        let _ = self.child.kill();
        let _ = self.child.wait();

        if let Some(handle) = self.outbound_handle.take() {
            let _ = handle.join();
        }

        if let Some(handle) = self.inbound_handle.take() {
            let _ = handle.join();
        }
    }
}

fn spawn_web_view_process(server_name: &str) -> Result<Child, String> {
    let mut command = Command::new("cargo");

    command
        .args([
            "run",
            "--features",
            "web_view_process",
            "--bin",
            "web_view_process",
            "--",
            server_name,
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    command
        .spawn()
        .map_err(|err| format!("failed to launch web-view process: {}", err))
}

fn pipe_child_logs(child: &mut Child) {
    if let Some(stdout) = child.stdout.take() {
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                println!("[web-view] {}", line);
            }
        });
    }

    if let Some(stderr) = child.stderr.take() {
        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                eprintln!("[web-view] {}", line);
            }
        });
    }
}

fn flush_queued_events(
    to_child: &IpcSender<Event>,
    queued_events: &Arc<Mutex<VecDeque<Event>>>,
) {
    let mut queue = match queued_events.lock() {
        Ok(queue) => queue,
        Err(err) => {
            error!("web-view queue lock poisoned: {}", err);
            return;
        }
    };

    while let Some(event) = queue.pop_front() {
        if let Err(err) = to_child.send(event) {
            warn!("failed to flush queued web-view event: {}", err);
            break;
        }
    }
}
