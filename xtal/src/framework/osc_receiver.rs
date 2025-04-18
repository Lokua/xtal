use nannou_osc as osc;
use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::thread;

use crate::runtime::global;

use super::prelude::*;

pub static SHARED_OSC_RECEIVER: LazyLock<Arc<Receiver>> = LazyLock::new(|| {
    let receiver = Receiver::new();
    if let Err(e) = receiver.start() {
        warn!("Failed to start shared OSC receiver: {}", e);
    }
    receiver
});

type OscCallback = Box<dyn Fn(&osc::Message) + Send + Sync>;

pub struct Receiver {
    callbacks: Arc<Mutex<HashMap<String, Vec<OscCallback>>>>,
    thread_handle: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
    thread_running: Arc<AtomicBool>,
}

impl Default for Receiver {
    fn default() -> Self {
        Self {
            callbacks: Arc::new(Mutex::new(HashMap::default())),
            thread_handle: Arc::new(Mutex::new(None)),
            thread_running: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Receiver {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn register_callback<F>(&self, address: &str, callback: F)
    where
        F: Fn(&osc::Message) + Send + Sync + 'static,
    {
        let mut callbacks = self.callbacks.lock().unwrap();
        let address_callbacks =
            callbacks.entry(address.to_string()).or_default();
        address_callbacks.push(Box::new(callback));
    }

    pub fn start(&self) -> Result<(), Box<dyn Error>> {
        let receiver = osc::Receiver::bind(global::osc_port())?;
        let callbacks = self.callbacks.clone();

        self.thread_running.store(true, Ordering::SeqCst);
        let running = self.thread_running.clone();
        let port_at_this_time = global::osc_port();

        let handle = thread::spawn(move || {
            while running.load(Ordering::SeqCst) {
                let mut processed = false;
                for (packet, _) in receiver.try_iter() {
                    processed = true;
                    if let osc::Packet::Message(msg) = packet {
                        let callbacks = callbacks.lock().unwrap();

                        if let Some(handlers) = callbacks.get(&msg.addr) {
                            for handler in handlers {
                                handler(&msg);
                            }
                        } else if let Some(handlers) = callbacks.get("*") {
                            for handler in handlers {
                                handler(&msg);
                            }
                        }
                    }
                }
                if !processed {
                    thread::yield_now();
                }
            }
            info!(
                "OSC receiver thread on port {} is exiting",
                port_at_this_time
            );
        });

        let mut thread_handle = self.thread_handle.lock().unwrap();
        *thread_handle = Some(handle);

        info!("OSC receiver listening on port {}", global::osc_port());

        Ok(())
    }

    pub fn stop(&self) -> Result<(), Box<dyn Error>> {
        self.thread_running.store(false, Ordering::SeqCst);
        let mut thread_handle = self.thread_handle.lock().unwrap();
        if let Some(handle) = thread_handle.take() {
            handle.join().unwrap();
        }
        Ok(())
    }

    pub fn restart(&self) -> Result<(), Box<dyn Error>> {
        self.stop()?;
        info!("Restarting...");
        self.start()
    }
}
