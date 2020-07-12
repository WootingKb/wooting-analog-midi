#![feature(vec_remove_item)]
extern crate ctrlc;
extern crate env_logger;
extern crate midir;
extern crate wooting_analog_wrapper;
#[macro_use]
extern crate lazy_static;

use log::{error, info, trace, warn};
use sdk::{DeviceInfo, HIDCodes, SDKResult, FromPrimitive};
use std::error::Error;
use std::io::{stdin, stdout, Write};
use std::thread::sleep;
use std::time::Duration;
use wooting_analog_wrapper as sdk;

use midir::{MidiOutput, MidiOutputPort, MidiOutputConnection};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::HashMap;
use std::borrow::{Borrow, BorrowMut};
use std::cmp::{max, min};
use std::f32::consts::E;

const DEVICE_BUFFER_MAX: usize = 5;
const ANALOG_BUFFER_READ_MAX: usize = 10;
const NOTE_ON_MSG: u8 = 0x90;
const NOTE_OFF_MSG: u8 = 0x80;
const POLY_AFTERTOUCH_MSG: u8 = 0xA0;
const VELOCITY: u8 = 0x64;
const THRESHOLD: f32 = 0.1;
const AFTERTOUCH: bool = true;
// How many times a second we'll check for updates on how much keys are pressed
const REFRESH_RATE: f32 = 100.0; //Hz

// NoteID Reference: https://newt.phys.unsw.edu.au/jw/notes.html
type NoteID = u8;
lazy_static! {
    static ref KEYMAPPING: HashMap<HIDCodes, NoteID> = {
        [
            (HIDCodes::Q, 57),
            (HIDCodes::W, 58),
            (HIDCodes::E, 59),
            (HIDCodes::R, 60),
            (HIDCodes::T, 61),
            (HIDCodes::Y, 62),
            (HIDCodes::U, 63),
            (HIDCodes::I, 64),
            (HIDCodes::O, 65),
            (HIDCodes::P, 66),
        ].iter().cloned().collect()
    };
}
trait NoteSink {
    // TODO: Return Result
    fn noteOn(&mut self, noteID: NoteID, velocity: f32);
    fn noteOff(&mut self, noteID: NoteID, velocity: f32);
    fn polyphonic_aftertouch(&mut self, noteID: NoteID, pressure: f32);
}

impl NoteSink for MidiOutputConnection {
    fn noteOn(&mut self, noteID: NoteID, velocity: f32)
    {
        let vbyte = (f32::min(velocity, 1.0) * 127.0) as u8;
        self.send(&[NOTE_ON_MSG, noteID, vbyte]);
    }

    fn noteOff(&mut self, noteID: NoteID, velocity: f32) {
        let vbyte = (f32::min(velocity, 1.0) * 127.0) as u8;
        self.send(&[NOTE_OFF_MSG, noteID, vbyte]);
    }

    fn polyphonic_aftertouch(&mut self, noteID: NoteID, pressure: f32) {
        self.send(&[POLY_AFTERTOUCH_MSG, noteID, (f32::min(pressure, 1.0) * 127.0) as u8]);
    }
}

pub struct Note {
    currentValue: f32,
    noteID: NoteID,
    // associatedKey: HIDCodes,
    pressed: bool,
    poly_aftertouching: bool,
    velocity: f32,
    pressure: f32,
}

impl Note {
    fn new(key: HIDCodes, note: NoteID) -> Note {
        Note {
            // associatedKey: key,
            noteID: note,
            currentValue: 0.0,
            pressed: false,
            poly_aftertouching: false,
            velocity: 0.0,
            pressure: 0.0,
        }
    }


    fn update_current_value(&mut self, newValue: f32, sink: &mut impl NoteSink) {
        self.velocity = f32::min(f32::max(f32::max(0.0, newValue - self.pressure) * 2.0, self.velocity * 0.9), 1.0);
        self.currentValue = newValue;
        self.pressure = newValue;

        if newValue > THRESHOLD {
            // 'Pressed'
            if !self.pressed {
                sink.noteOn(self.noteID, self.velocity);
                self.pressed = true;
            } else {
                // While we are in the range of what we consider 'pressed' for the key & the note on has already been sent we send aftertouch
                if AFTERTOUCH {
                    sink.polyphonic_aftertouch(self.noteID, self.pressure);
                }
            }
        } else {
            // 'Not Pressed'
            if self.pressed {
                sink.noteOff(self.noteID, self.velocity);
                self.pressed = false;
            }
        }
    }
}


fn main() {
    match run() {
        Ok(_) => (),
        Err(err) => error!("Error: {}", err),
    }
}


fn generateNoteMapping(keymapping: &HashMap<HIDCodes, u8>) -> HashMap<HIDCodes, Note> {
    keymapping.iter().map(|(key, note)| (key.clone(), Note::new(key.clone(), *note))).collect()
}

fn run() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    info!("Starting Wooting Analog SDK!");
    let init_result: SDKResult<u32> = sdk::initialise();

    match init_result.0 {
        Ok(device_num) => {
            info!("SDK Successfully initialised with {} devices", device_num);
            let devices: Vec<DeviceInfo> = sdk::get_connected_devices_info(DEVICE_BUFFER_MAX)
                .0
                .unwrap();
            assert_eq!(device_num, devices.len() as u32);
            for (i, device) in devices.iter().enumerate() {
                println!("Device {} is {:?}", i, device);
            }
        }
        Err(e) => {
            return Err(format!("SDK Failed to initialise. Error: {:?}", e).into());
        }
    }

    let mut notes = generateNoteMapping(&*KEYMAPPING);

    let midi_out = MidiOutput::new("My Test Output")?;
    // Get an output port (read from console if multiple are available)
    let out_ports = midi_out.ports();
    let out_port: &MidiOutputPort = match out_ports.len() {
        0 => return Err("no output port found".into()),
        1 => {
            println!(
                "Choosing the only available output port: {}",
                midi_out.port_name(&out_ports[0]).unwrap()
            );
            &out_ports[0]
        }
        _ => {
            println!("\nAvailable output ports:");
            for (i, p) in out_ports.iter().enumerate() {
                println!("{}: {}", i, midi_out.port_name(p).unwrap());
            }
            print!("Please select output port: ");
            stdout().flush()?;
            let mut input = String::new();
            stdin().read_line(&mut input)?;
            out_ports
                .get(input.trim().parse::<usize>()?)
                .ok_or("invalid output port selected")?
        }
    };
    info!("\nOpening connection");
    let mut conn_out = midi_out.connect(out_port, "midir-test")?;
    info!("Connection open. Listen!");

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
        .expect("Error setting Ctrl-C handler");

    // let mut pressed_keys: Vec<u16> = Vec::new();


    while running.load(Ordering::SeqCst) {
        let read_result: SDKResult<HashMap<u16, f32>> =
            sdk::read_full_buffer(ANALOG_BUFFER_READ_MAX);
        match read_result.0 {
            Ok(analog_data) => {
                for (code, value) in analog_data.iter() {
                    if let Some(hid_code) = HIDCodes::from_u16(*code) {
                        if let Some(note) = notes.get_mut(&hid_code) {
                            note.update_current_value(*value, conn_out.borrow_mut());
                        }
                    }
                }
            }
            Err(e) => {
                error!("Error reading full buffer, {:?}", e);
            }
        };
        sleep(Duration::from_secs_f32(1.0 / REFRESH_RATE))
    }

    info!("\nClosing connection");
    // This is optional, the connection would automatically be closed as soon as it goes out of scope
    conn_out.close();
    sdk::uninitialise();
    info!("Connection closed");
    Ok(())
}
