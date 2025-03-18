use nannou_osc as osc;
use rustc_hash::FxHashMap;
use std::sync::{Arc, Mutex};
use std::thread;

use super::prelude::*;

lazy_static::lazy_static! {
    pub static ref SHARED_OSC_RECEIVER: Arc<Receiver> = {
        let receiver = Receiver::new();
        if let Err(e) = receiver.start() {
            warn!("Failed to start shared OSC receiver: {}", e);
        }
        receiver
    };
}

type OscCallback = Box<dyn Fn(&osc::Message) + Send + Sync>;

#[derive(Default)]
pub struct Receiver {
    callbacks: Arc<Mutex<FxHashMap<String, Vec<OscCallback>>>>,
}

impl Receiver {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            callbacks: Arc::new(Mutex::new(FxHashMap::default())),
        })
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

    pub fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let receiver = osc::Receiver::bind(crate::config::OSC_PORT)?;
        let callbacks = self.callbacks.clone();

        thread::spawn(move || {
            for (packet, _) in receiver.iter() {
                if let osc::Packet::Message(msg) = packet {
                    let callbacks = callbacks.lock().unwrap();

                    if let Some(handlers) = callbacks.get(&msg.addr) {
                        for handler in handlers {
                            handler(&msg);
                        }
                    }
                }
            }
        });

        info!("OSC receiver listening on port {}", crate::config::OSC_PORT);

        Ok(())
    }
}
