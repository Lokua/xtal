use midir::Ignore;
use midir::MidiInput;
use midir::MidiInputConnection;
use midir::MidiOutput;
use midir::MidiOutputConnection;
use std::error::Error;
use std::fmt;
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::Mutex;
use std::thread;

use super::prelude::*;

static THREADS: LazyLock<
    Mutex<HashMap<ConnectionType, thread::JoinHandle<()>>>,
> = LazyLock::new(|| Mutex::new(HashMap::default()));

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ConnectionType {
    Clock,
    Control,
    GlobalStartStop,
    Mapping,
}

impl fmt::Display for ConnectionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConnectionType::Clock => write!(f, "Clock"),
            ConnectionType::Control => write!(f, "Control"),
            ConnectionType::GlobalStartStop => write!(f, "GlobalStartStop"),
            ConnectionType::Mapping => write!(f, "Mapping"),
        }
    }
}

pub fn on_message<F>(
    connection_type: ConnectionType,
    port: &str,
    callback: F,
) -> Result<(), Box<dyn Error>>
where
    F: Fn(u64, &[u8]) + Send + Sync + 'static,
{
    let midi_in = MidiInput::new(&connection_type.to_string())?;
    let port = port.to_string();

    let in_ports = midi_in.ports();
    let in_port = in_ports
        .iter()
        .find(|p| midi_in.port_name(p).unwrap_or_default() == port)
        .ok_or_else(|| format!("Unable to find input port: {}", port))?
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
                move |stamp, message, _| {
                    trace!("MIDI message: {}, {:?}", stamp, message);
                    callback(stamp, message);
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

pub fn disconnect(connection_type: ConnectionType) {
    let mut threads = THREADS.lock().unwrap();
    if let Some(handle) = threads.remove(&connection_type) {
        info!("[disconnect] Unparking {} thread", connection_type);
        handle.thread().unpark();
    }
}

#[allow(dead_code)]
pub struct MidiOut {
    port: String,
    connection: Option<MidiOutputConnection>,
}

impl MidiOut {
    pub fn new(port: &str) -> Self {
        Self {
            port: port.to_string(),
            connection: None,
        }
    }

    pub fn connect(&mut self) -> Result<(), Box<dyn Error>> {
        let midi_out = MidiOutput::new("ControlOut")?;
        let out_ports = midi_out.ports();
        let out_port = out_ports
            .iter()
            .find(|p| midi_out.port_name(p).unwrap_or_default() == self.port)
            .ok_or_else(|| {
                format!("Unable to find output port: {}", self.port)
            })?;
        let connection = midi_out.connect(out_port, "ControlOut")?;
        self.connection = Some(connection);
        Ok(())
    }

    pub fn send(&mut self, message: &[u8; 3]) -> Result<(), Box<dyn Error>> {
        if let Some(connection) = &mut self.connection {
            connection.send(message)?
        } else {
            warn!("Midi ControlOut connection has not been established");
        }
        Ok(())
    }
}

pub type PortIndexAndName = (usize, String);

pub fn list_input_ports() -> Result<Vec<PortIndexAndName>, Box<dyn Error>> {
    let mut midi_in = MidiInput::new("midir_test_input")?;
    midi_in.ignore(Ignore::None);
    let mut ports = vec![];
    for (i, p) in midi_in.ports().iter().enumerate() {
        ports.push((i, midi_in.port_name(p)?))
    }
    Ok(ports)
}

pub fn list_output_ports() -> Result<Vec<PortIndexAndName>, Box<dyn Error>> {
    let midi_out = MidiOutput::new("midir_test_output")?;
    let mut ports = vec![];
    for (i, p) in midi_out.ports().iter().enumerate() {
        ports.push((i, midi_out.port_name(p)?))
    }
    Ok(ports)
}

pub fn print_ports() -> Result<(), Box<dyn Error>> {
    println!("\nAvailable input ports:");
    for (index, port_name) in list_input_ports()? {
        println!("    {}: {}", index, port_name);
    }

    println!("\nAvailable output ports:");
    for (index, port_name) in list_output_ports()? {
        println!("    {}: {}", index, port_name);
    }

    println!();

    Ok(())
}

pub fn is_control_change(status: u8) -> bool {
    status & 0xF0 == 0xB0
}
