#![feature(vec_remove_item)]
// extern crate ctrlc;
extern crate midir;
extern crate wooting_analog_wrapper;
#[macro_use]
extern crate lazy_static;

use log::{error, info};
use sdk::SDKResult;
pub use sdk::{DeviceInfo, FromPrimitive, HIDCodes};
use std::error::Error;
use wooting_analog_wrapper as sdk;

use midir::{MidiOutput, MidiOutputConnection, MidiOutputPort};
use std::collections::HashMap;

const DEVICE_BUFFER_MAX: usize = 5;
const ANALOG_BUFFER_READ_MAX: usize = 10;
const NOTE_ON_MSG: u8 = 0x90;
const NOTE_OFF_MSG: u8 = 0x80;
const POLY_AFTERTOUCH_MSG: u8 = 0xA0;
// const VELOCITY: u8 = 0x64;
const THRESHOLD: f32 = 0.1;
const AFTERTOUCH: bool = true;
// How many times a second we'll check for updates on how much keys are pressed
pub const REFRESH_RATE: f32 = 100.0; //Hz

// NoteID Reference: https://newt.phys.unsw.edu.au/jw/notes.html
type NoteID = u8;
// lazy_static! {
//     static ref KEYMAPPING: HashMap<HIDCodes, NoteID> = {
//         [
//             (HIDCodes::Q, 57),
//             (HIDCodes::W, 58),
//             (HIDCodes::E, 59),
//             (HIDCodes::R, 60),
//             (HIDCodes::T, 61),
//             (HIDCodes::Y, 62),
//             (HIDCodes::U, 63),
//             (HIDCodes::I, 64),
//             (HIDCodes::O, 65),
//             (HIDCodes::P, 66),
//         ]
//         .iter()
//         .cloned()
//         .collect()
//     };
// }
trait NoteSink {
    // TODO: Return Result
    fn note_on(&mut self, note_id: NoteID, velocity: f32);
    fn note_off(&mut self, note_id: NoteID, velocity: f32);
    fn polyphonic_aftertouch(&mut self, note_id: NoteID, pressure: f32);
}

impl NoteSink for MidiOutputConnection {
    fn note_on(&mut self, note_id: NoteID, velocity: f32) {
        let vbyte = (f32::min(velocity, 1.0) * 127.0) as u8;
        self.send(&[NOTE_ON_MSG, note_id, vbyte]);
    }

    fn note_off(&mut self, note_id: NoteID, velocity: f32) {
        let vbyte = (f32::min(velocity, 1.0) * 127.0) as u8;
        self.send(&[NOTE_OFF_MSG, note_id, vbyte]);
    }

    fn polyphonic_aftertouch(&mut self, note_id: NoteID, pressure: f32) {
        self.send(&[
            POLY_AFTERTOUCH_MSG,
            note_id,
            (f32::min(pressure, 1.0) * 127.0) as u8,
        ]);
    }
}

#[derive(Debug)]
pub struct Note {
    pub current_value: f32,
    pub note_id: Option<NoteID>,
    associated_key: HIDCodes,
    pressed: bool,
    velocity: f32,
    pressure: f32,
}

impl Note {
    pub fn new(key: HIDCodes, note: Option<NoteID>) -> Note {
        Note {
            associated_key: key,
            note_id: note,
            current_value: 0.0,
            pressed: false,
            velocity: 0.0,
            pressure: 0.0,
        }
    }

    fn update_note(&mut self, note: Option<NoteID>, sink: Option<&mut impl NoteSink>) {
        if let Some(sink) = sink {
            if let Some(current_note) = self.note_id {
                if self.pressed {
                    sink.note_off(current_note, self.velocity);
                    self.pressed = false;
                }
            }
        }

        self.note_id = note;
    }

    fn update_current_value(&mut self, new_value: f32, sink: &mut impl NoteSink) {
        self.velocity = f32::min(
            f32::max(
                f32::max(0.0, new_value - self.pressure) * 2.0,
                self.velocity * 0.9,
            ),
            1.0,
        );
        self.current_value = new_value;
        self.pressure = new_value;

        if let Some(note_id) = self.note_id {
            if new_value > THRESHOLD {
                // 'Pressed'
                if !self.pressed {
                    sink.note_on(note_id, self.velocity);
                    self.pressed = true;
                } else {
                    // While we are in the range of what we consider 'pressed' for the key & the note on has already been sent we send aftertouch
                    if AFTERTOUCH {
                        sink.polyphonic_aftertouch(note_id, self.pressure);
                    }
                }
            } else {
                // 'Not Pressed'
                if self.pressed {
                    sink.note_off(note_id, self.velocity);
                    self.pressed = false;
                }
            }
        }
    }
}

// fn generate_note_mapping(_keymapping: &HashMap<HIDCodes, u8>) -> HashMap<HIDCodes, Note> {
fn generate_note_mapping() -> HashMap<HIDCodes, Note> {
    // keymapping
    //     .iter()
    //     .map(|(key, note)| (key.clone(), Note::new(key.clone(), *note)))
    //     .collect()
    (0..255)
        .step_by(1)
        .map(|code| HIDCodes::from_u8(code as u8))
        .filter(|code| code.is_some())
        .map(|code| code.unwrap())
        .map(|code| (code.clone(), Note::new(code, None)))
        .collect()
}

pub struct MidiService {
    connection: Option<MidiOutputConnection>,
    pub notes: HashMap<HIDCodes, Note>,
}

//TODO: Determine if this is safe or a different solution is required
unsafe impl Send for MidiService {}
unsafe impl Sync for MidiService {}

impl MidiService {
    pub fn new() -> Self {
        MidiService {
            connection: None,
            notes: generate_note_mapping(),
        }
    }

    pub fn update_mapping(
        &mut self,
        mapping: &HashMap<HIDCodes, NoteID>,
    ) -> Result<(), Box<dyn Error>> {
        // self.notes =
        for (key, note) in self.notes.iter_mut() {
            if let Some(note_id) = mapping.get(&key) {
                note.update_note(Some(*note_id), self.connection.as_mut());
            } else {
                note.update_note(None, self.connection.as_mut())
            }
        }

        Ok(())
    }

    pub fn init(&mut self) -> Result<(), Box<dyn Error>> {
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

        let midi_out = MidiOutput::new("Wooting Analog MIDI Output")?;
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
                // println!("\nAvailable output ports:");
                // for (i, p) in out_ports.iter().enumerate() {
                //     println!("{}: {}", i, midi_out.port_name(p).unwrap());
                // }
                // print!("Please select output port: ");
                // stdout().flush()?;
                // let mut input = String::new();
                // stdin().read_line(&mut input)?;
                // out_ports
                //     .get(input.trim().parse::<usize>()?)
                // .ok_or("invalid output port selected")?
                out_ports.get(0).ok_or("invalid output port selected")?
            }
        };
        info!("Opening connection");
        self.connection = Some(midi_out.connect(out_port, "midir-test")?);

        Ok(())
    }

    pub fn poll(&mut self) -> Result<(), Box<dyn Error>> {
        if self.connection.is_none() {
            Err("No MIDI connection!")?;
        }

        let read_result: SDKResult<HashMap<u16, f32>> =
            sdk::read_full_buffer(ANALOG_BUFFER_READ_MAX);
        match read_result.0 {
            Ok(analog_data) => {
                for (code, value) in analog_data.iter() {
                    if let Some(hid_code) = HIDCodes::from_u16(*code) {
                        if let Some(note) = self.notes.get_mut(&hid_code) {
                            note.update_current_value(*value, self.connection.as_mut().unwrap());
                        }
                    }
                }
            }
            Err(e) => {
                error!("Error reading full buffer, {:?}", e);
            }
        };
        Ok(())
    }

    pub fn uninit(&mut self) {
        info!("Uninitialising MidiService");
        sdk::uninitialise();
        if let Some(output) = self.connection.take() {
            output.close();
        }
    }
}
impl Drop for MidiService {
    fn drop(&mut self) {
        self.uninit();
    }
}
