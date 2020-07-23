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
// The analog threshold at which we consider a note being turned on
const THRESHOLD: f32 = 0.1;
// What counts as a key being pressed. Currently used for modifier press detection
const ACTUATION_POINT: f32 = 0.2;
const MODIFIER_KEY: HIDCodes = HIDCodes::LeftShift;
const MODIFIER_NOTE_SHIFT: u8 = 1;
const AFTERTOUCH: bool = true;
// How many times a second we'll check for updates on how much keys are pressed
pub const REFRESH_RATE: f32 = 100.0; //Hz

// NoteID Reference: https://newt.phys.unsw.edu.au/jw/notes.html
pub type NoteID = u8;
pub type Channel = u8;

trait NoteSink {
    fn note_on(&mut self, note_id: NoteID, velocity: f32, channel: Channel) -> Result<()>;
    fn note_off(&mut self, note_id: NoteID, velocity: f32, channel: Channel) -> Result<()>;
    fn polyphonic_aftertouch(
        &mut self,
        note_id: NoteID,
        pressure: f32,
        channel: Channel,
    ) -> Result<()>;
}

impl NoteSink for MidiOutputConnection {
    fn note_on(&mut self, note_id: NoteID, velocity: f32, channel: Channel) -> Result<()> {
        let vbyte = (f32::min(velocity, 1.0) * 127.0) as u8;
        self.send(&[NOTE_ON_MSG | channel, note_id, vbyte])?;
        Ok(())
    }

    fn note_off(&mut self, note_id: NoteID, velocity: f32, channel: Channel) -> Result<()> {
        let vbyte = (f32::min(velocity, 1.0) * 127.0) as u8;
        self.send(&[NOTE_OFF_MSG | channel, note_id, vbyte])?;
        Ok(())
    }

    fn polyphonic_aftertouch(
        &mut self,
        note_id: NoteID,
        pressure: f32,
        channel: Channel,
    ) -> Result<()> {
        self.send(&[
            POLY_AFTERTOUCH_MSG | channel,
            note_id,
            (f32::min(pressure, 1.0) * 127.0) as u8,
        ])?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct Note {
    pub note_id: NoteID,
    pub pressed: bool,
    modifier_active: bool,
    pub velocity: f32,
    pub channel: Channel,
}

impl Note {
    pub fn new(channel: Channel, note: NoteID) -> Note {
        Note {
            note_id: note,
            pressed: false,
            velocity: 0.0,
            modifier_active: false,
            channel,
        }
    }

    fn get_effective_note(&self) -> NoteID {
        self.note_id
            + (if self.modifier_active {
                MODIFIER_NOTE_SHIFT
            } else {
                0
            })
    }

    fn update_current_value(
        &mut self,
        previous_value: f32,
        new_value: f32,
        sink: &mut impl NoteSink,
        modifer_pressed: bool,
    ) -> Result<()> {
        self.velocity = f32::min(
            f32::max(
                f32::max(0.0, new_value - previous_value) * 2.0,
                self.velocity * 0.9,
            ),
            1.0,
        );
        // If the modifier pressed state has changed we need to make sure we turn the current note off because the note id will be changed
        if modifer_pressed != self.modifier_active {
            if self.pressed {
                sink.note_off(self.get_effective_note(), self.velocity, self.channel)?;
                self.pressed = false;
            }
        }
        self.modifier_active = modifer_pressed;

        if new_value > THRESHOLD {
            // 'Pressed'
            if !self.pressed {
                sink.note_on(self.get_effective_note(), self.velocity, self.channel)?;
                self.pressed = true;
            } else {
                // While we are in the range of what we consider 'pressed' for the key & the note on has already been sent we send aftertouch
                if AFTERTOUCH {
                    sink.polyphonic_aftertouch(self.get_effective_note(), new_value, self.channel)?;
                }
            }
        } else {
            // 'Not Pressed'
            if self.pressed {
                sink.note_off(self.get_effective_note(), self.velocity, self.channel)?;
                self.pressed = false;
            }
        }

        Ok(())
    }

    fn drop(&mut self, sink: &mut Option<impl NoteSink>) -> Result<()> {
        if let Some(sink) = sink {
            if self.pressed {
                sink.note_off(self.get_effective_note(), self.velocity, self.channel)?;
                self.pressed = false;
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct Key {
    pub notes: Vec<Note>,
    pub current_value: f32,
}

impl Key {
    fn new() -> Self {
        Self {
            notes: vec![],
            current_value: 0.0,
        }
    }

    fn update_value(
        &mut self,
        new_value: f32,
        sink: &mut impl NoteSink,
        modifer_pressed: bool,
    ) -> Result<()> {
        for note in self.notes.iter_mut() {
            note.update_current_value(self.current_value, new_value, sink, modifer_pressed)?;
        }

        self.current_value = new_value;

        Ok(())
    }

    fn update_mappings(
        &mut self,
        mappings: &Vec<(Channel, NoteID)>,
        sink: &mut Option<impl NoteSink>,
    ) -> Result<()> {
        for mut note in self.notes.drain(..) {
            note.drop(sink)?;
        }

        for (channel, note_id) in mappings.iter() {
            self.notes.push(Note::new(*channel, *note_id));
        }

        Ok(())
    }
}

fn generate_note_mapping() -> HashMap<HIDCodes, Key> {
    (0..255)
        .step_by(1)
        .map(|code| HIDCodes::from_u8(code as u8))
        .filter(|code| code.is_some())
        .map(|code| code.unwrap())
        .map(|code| (code.clone(), Key::new()))
        .collect()
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PortOption(usize, String, bool);

pub struct MidiService {
    pub port_options: Option<Vec<PortOption>>,
    connection: Option<MidiOutputConnection>,
    pub keys: HashMap<HIDCodes, Key>,
}

//TODO: Determine if this is safe (LUL imagine saying it may be safe when it literally says unsafe) or a different solution is required
// Tbf haven't ran into any issues yet, so might be okay
unsafe impl Send for MidiService {}
unsafe impl Sync for MidiService {}

impl MidiService {
    pub fn new() -> Self {
        MidiService {
            port_options: None,
            connection: None,
            keys: generate_note_mapping(),
        }
    }

    pub fn update_mapping(
        &mut self,
        mapping: &HashMap<HIDCodes, Vec<(Channel, NoteID)>>,
    ) -> Result<()> {
        // self.notes =
        let empty_mapping = vec![];
        for (key_id, key) in self.keys.iter_mut() {
            if let Some(mappings) = mapping.get(&key_id) {
                key.update_mappings(mappings, &mut self.connection)?;
            } else {
                key.update_mappings(&empty_mapping, &mut self.connection)?;
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
        //TODO: Deal with the case where the port list has changed since the `port_options` was generated
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
                let modifier_pressed =
                    (*analog_data.get(&(MODIFIER_KEY as u16)).unwrap_or(&0.0)) >= ACTUATION_POINT;
                for (code, value) in analog_data.iter() {
                    if let Some(hid_code) = HIDCodes::from_u16(*code) {
                        if let Some(key) = self.keys.get_mut(&hid_code) {
                            key.update_value(
                                *value,
                                self.connection.as_mut().unwrap(),
                                modifier_pressed,
                            )?;
                        }
                    }
                }
            }
            Err(e) => {
                bail!("Error reading full buffer, {:?}", e);
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
