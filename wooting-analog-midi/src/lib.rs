#![feature(vec_remove_item)]
// extern crate ctrlc;
extern crate midir;
extern crate wooting_analog_wrapper;
#[allow(unused_imports)]
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate anyhow;

#[allow(unused_imports)]
use log::{error, info};
use sdk::SDKResult;
pub use sdk::{DeviceInfo, FromPrimitive, HIDCodes};
use wooting_analog_wrapper as sdk;

use anyhow::Result;
use midir::{MidiOutput, MidiOutputConnection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const DEVICE_BUFFER_MAX: usize = 5;
const ANALOG_BUFFER_READ_MAX: usize = 40;
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
    fn note_on(&mut self, note_id: NoteID, velocity: f32) -> Result<()>;
    fn note_off(&mut self, note_id: NoteID, velocity: f32) -> Result<()>;
    fn polyphonic_aftertouch(&mut self, note_id: NoteID, pressure: f32) -> Result<()>;
}

impl NoteSink for MidiOutputConnection {
    fn note_on(&mut self, note_id: NoteID, velocity: f32) -> Result<()> {
        let vbyte = (f32::min(velocity, 1.0) * 127.0) as u8;
        self.send(&[NOTE_ON_MSG, note_id, vbyte])?;
        Ok(())
    }

    fn note_off(&mut self, note_id: NoteID, velocity: f32) -> Result<()> {
        let vbyte = (f32::min(velocity, 1.0) * 127.0) as u8;
        self.send(&[NOTE_OFF_MSG, note_id, vbyte])?;
        Ok(())
    }

    fn polyphonic_aftertouch(&mut self, note_id: NoteID, pressure: f32) -> Result<()> {
        self.send(&[
            POLY_AFTERTOUCH_MSG,
            note_id,
            (f32::min(pressure, 1.0) * 127.0) as u8,
        ])?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct Note {
    pub current_value: f32,
    pub note_id: Option<NoteID>,
    associated_key: HIDCodes,
    pressed: bool,
    velocity: f32,
}

impl Note {
    pub fn new(key: HIDCodes, note: Option<NoteID>) -> Note {
        Note {
            associated_key: key,
            note_id: note,
            current_value: 0.0,
            pressed: false,
            velocity: 0.0,
        }
    }

    fn update_note(
        &mut self,
        note: Option<NoteID>,
        sink: Option<&mut impl NoteSink>,
    ) -> Result<()> {
        if let Some(sink) = sink {
            if let Some(current_note) = self.note_id {
                if self.pressed {
                    sink.note_off(current_note, self.velocity)?;
                    self.pressed = false;
                }
            }
        }

        self.note_id = note;
        Ok(())
    }

    fn update_current_value(&mut self, new_value: f32, sink: &mut impl NoteSink) -> Result<()> {
        self.velocity = f32::min(
            f32::max(
                f32::max(0.0, new_value - self.current_value) * 2.0,
                self.velocity * 0.9,
            ),
            1.0,
        );
        self.current_value = new_value;

        if let Some(note_id) = self.note_id {
            if new_value > THRESHOLD {
                // 'Pressed'
                if !self.pressed {
                    sink.note_on(note_id, self.velocity)?;
                    self.pressed = true;
                } else {
                    // While we are in the range of what we consider 'pressed' for the key & the note on has already been sent we send aftertouch
                    if AFTERTOUCH {
                        sink.polyphonic_aftertouch(note_id, self.current_value)?;
                    }
                }
            } else {
                // 'Not Pressed'
                if self.pressed {
                    sink.note_off(note_id, self.velocity)?;
                    self.pressed = false;
                }
            }
        }
        Ok(())
    }
}

fn generate_note_mapping() -> HashMap<HIDCodes, Note> {
    (0..255)
        .step_by(1)
        .map(|code| HIDCodes::from_u8(code as u8))
        .filter(|code| code.is_some())
        .map(|code| code.unwrap())
        .map(|code| (code.clone(), Note::new(code, None)))
        .collect()
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PortOption(usize, String, bool);

pub struct MidiService {
    pub port_options: Option<Vec<PortOption>>,
    connection: Option<MidiOutputConnection>,
    pub notes: HashMap<HIDCodes, Note>,
}

//TODO: Determine if this is safe or a different solution is required
unsafe impl Send for MidiService {}
unsafe impl Sync for MidiService {}

impl MidiService {
    pub fn new() -> Self {
        MidiService {
            port_options: None,
            connection: None,
            notes: generate_note_mapping(),
        }
    }

    pub fn update_mapping(&mut self, mapping: &HashMap<HIDCodes, NoteID>) -> Result<()> {
        // self.notes =
        for (key, note) in self.notes.iter_mut() {
            if let Some(note_id) = mapping.get(&key) {
                note.update_note(Some(*note_id), self.connection.as_mut())?;
            } else {
                note.update_note(None, self.connection.as_mut())?;
            }
        }

        Ok(())
    }

    // pub fn init(&mut self, connection_preference: Option<usize>) -> Result<(), Box<dyn Error>> {
    pub fn init(&mut self) -> Result<()> {
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
                Err(anyhow!("SDK Failed to initialise. Error: {:?}", e))?;
            }
        }

        let midi_out = MidiOutput::new("Wooting Analog MIDI Output")?;

        let ports = midi_out.ports();
        self.port_options = Some(
            ports
                .iter()
                .enumerate()
                .map(|(i, port)| PortOption(i, midi_out.port_name(&port).unwrap(), i == 0))
                .collect(),
        );
        info!("We have {} ports available!", ports.len());
        if ports.len() > 0 {
            info!("Opening connection");
            self.connection = Some(
                midi_out
                    .connect(&ports[0], "wooting-analog-midi")
                    .map_err(|e| anyhow!("Error: {}", e))?,
            );
        } else {
            info!("No output ports available!");
        }
        // self.port_options = Some(midi_out);
        Ok(())
    }

    pub fn select_port(&mut self, option: usize) -> Result<()> {
        if let Some(options) = &self.port_options {
            if option >= options.len() {
                return Err(anyhow!("Port option out of range!"));
            }

            let selection = &options[option];

            // Port is already the selected one, don't need to do anything
            if selection.2 {
                return Ok(());
            }

            // Close previous connection in advance
            // if let Some(old) = self.connection.take() {
            //     old.close();
            // }

            let midi_out = MidiOutput::new("Wooting Analog MIDI Output")?;
            let ports = midi_out.ports();
            self.port_options = Some(
                ports
                    .iter()
                    .enumerate()
                    .map(|(i, port)| PortOption(i, midi_out.port_name(&port).unwrap(), i == option))
                    .collect(),
            );

            self.connection = Some(
                midi_out
                    .connect(&ports[option], "wooting-analog-midi")
                    .map_err(|e| anyhow!("Error: {}", e))?,
            );

            // for (i, port) in ports.iter().enumerate() {
            //     let port_name = midi_out.port_name(&port)?;

            //     if port_name == selection.1 {
            //     } else {

            //     }
            // }

            Ok(())
        } else {
            return Err(anyhow!("Port options not initialised"));
        }
    }

    pub fn poll(&mut self) -> Result<()> {
        if self.connection.is_none() {
            Err(anyhow!("No MIDI connection!"))?;
        }

        let read_result: SDKResult<HashMap<u16, f32>> =
            sdk::read_full_buffer(ANALOG_BUFFER_READ_MAX);
        match read_result.0 {
            Ok(analog_data) => {
                for (code, value) in analog_data.iter() {
                    if let Some(hid_code) = HIDCodes::from_u16(*code) {
                        if let Some(note) = self.notes.get_mut(&hid_code) {
                            note.update_current_value(*value, self.connection.as_mut().unwrap())?;
                        }
                    }
                }
            }
            Err(e) => {
                Err(anyhow!("Error reading full buffer, {:?}", e))?;
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
