use lazy_static::lazy_static;
use midir::Ignore;
use midir::MidiInput;
use midir::MidiInputConnection;
use midir::MidiOutput;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

use super::prelude::*;

lazy_static! {
    static ref THREADS: Mutex<HashMap<ConnectionType, thread::JoinHandle<()>>> =
        Mutex::new(HashMap::new());
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ConnectionType {
    Clock,
    Control,
    GlobalStartStop,
}

impl fmt::Display for ConnectionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConnectionType::Clock => write!(f, "Clock"),
            ConnectionType::Control => write!(f, "Control"),
            ConnectionType::GlobalStartStop => write!(f, "GlobalStartStop"),
        }
    }
}

pub fn on_message<F>(
    connection_type: ConnectionType,
    port: &str,
    callback: F,
) -> Result<(), Box<dyn Error>>
where
    F: Fn(&[u8]) + Send + Sync + 'static,
{
    let midi_in = MidiInput::new(&connection_type.to_string())?;
    let port = port.to_string();

    let in_ports = midi_in.ports();
    let in_port = in_ports
        .iter()
        .find(|p| midi_in.port_name(p).unwrap_or_default() == port)
        .expect("Unable to find input port")
        .clone();

    {
        let mut threads = THREADS.lock().unwrap();
        if let Some(handle) = threads.remove(&connection_type) {
            info!("Unparking {} ({}) thread", connection_type, port);
            handle.thread().unpark();
        }
    }

    let connection: Arc<Mutex<Option<MidiInputConnection<()>>>> =
        Arc::new(Mutex::new(None));
    let connection_clone = connection.clone();
    let connection_name = connection_type.to_string();
    let connection_type_clone = connection_type.clone();

    let handle = thread::spawn(move || {
        let conn_in = midi_in
            .connect(
                &in_port,
                &connection_name,
                move |_stamp, message, _| {
                    trace!("MIDI message: {:?}", message);
                    callback(message);
                },
                (),
            )
            .expect("Unable to connect");

        *connection_clone.lock().unwrap() = Some(conn_in);

        {
            info!(
                "Connected: {} ({}); connection count: {}",
                connection_type,
                port,
                THREADS.lock().unwrap().len()
            );
        }

        thread::park();

        if let Some(conn) = connection_clone.lock().unwrap().take() {
            drop(conn);
        }
    });

    {
        THREADS
            .lock()
            .unwrap()
            .insert(connection_type_clone, handle);
    }

    Ok(())
}

pub fn print_ports() -> Result<(), Box<dyn Error>> {
    let mut midi_in = MidiInput::new("midir test input")?;
    midi_in.ignore(Ignore::None);
    let midi_out = MidiOutput::new("midir test output")?;

    println!("\nAvailable input ports:");
    for (i, p) in midi_in.ports().iter().enumerate() {
        println!("    {}: {}", i, midi_in.port_name(p)?);
    }

    println!("\nAvailable output ports:");
    for (i, p) in midi_out.ports().iter().enumerate() {
        println!("    {}: {}", i, midi_out.port_name(p)?);
    }

    println!("");

    Ok(())
}
