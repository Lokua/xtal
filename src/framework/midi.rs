use midir::Ignore;
use midir::MidiInput;
use midir::MidiOutput;
use std::error::Error;
use std::thread;

use super::prelude::*;

pub fn on_message<F>(
    connection_name: &str,
    port: &str,
    callback: F,
) -> Result<(), Box<dyn Error>>
where
    F: Fn(&[u8]) + Send + Sync + 'static,
{
    let midi_in = MidiInput::new(connection_name)?;
    let port = port.to_string();

    let in_ports = midi_in.ports();
    let in_port = in_ports
        .iter()
        .find(|p| midi_in.port_name(p).unwrap_or_default() == port)
        .expect("Unable to find input port")
        .clone();

    let connection_name = connection_name.to_string();
    thread::spawn(move || {
        // _conn_in needs to be a named parameter so it's kept alive until the
        // end of the scope
        let _conn_in = midi_in
            .connect(
                &in_port,
                &format!("{} Read", connection_name),
                move |_stamp, message, _| {
                    trace!("MIDI message: {:?}", message);
                    callback(message);
                },
                (),
            )
            .expect("Unable to connect");

        info!(
            "Connected to {}, connection_name: {}",
            port, connection_name
        );

        thread::park();
    });

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
