extern crate midir;
extern crate wooting_analog_wrapper;
#[allow(unused_imports)]
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate anyhow;

use log::*;
use sdk::SDKResult;
pub use sdk::{DeviceInfo, FromPrimitive, HIDCodes, ToPrimitive, WootingAnalogResult};
use wooting_analog_wrapper as sdk;

use anyhow::{Context, Result};
use midir::{MidiOutput, MidiOutputConnection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

const DEVICE_BUFFER_MAX: usize = 5;
const ANALOG_BUFFER_READ_MAX: usize = 40;
const NOTE_ON_MSG: u8 = 0x90;
const NOTE_OFF_MSG: u8 = 0x80;
const POLY_AFTERTOUCH_MSG: u8 = 0xA0;
// const VELOCITY: u8 = 0x64;
// The analog threshold at which we consider a note being turned on
// const THRESHOLD: f32 = 0.5;
// What counts as a key being pressed. Currently used for modifier press detection
const ACTUATION_POINT: f32 = 0.2;
const MODIFIER_KEY: HIDCodes = HIDCodes::LeftShift;
const AFTERTOUCH: bool = true;
// How many times a second we'll check for updates on how much keys are pressed
pub const REFRESH_RATE: f32 = 100.0; //Hz
const MIDI_NOTE_MAX: u8 = 108;
const MIDI_NOTE_MIN: u8 = 21;

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

fn default_threshold() -> f32 {
    0.5
}

fn default_velocity_scale() -> f32 {
    5.0
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NoteConfig {
    #[serde(default = "default_threshold")]
    threshold: f32,
    #[serde(default = "default_velocity_scale")]
    velocity_scale: f32,
    // Any new properties should have a default added to it to ensure old configs get pulled in properly
}

impl NoteConfig {
    pub fn new(threshold: f32, velocity_scale: f32) -> Self {
        NoteConfig {
            threshold,
            velocity_scale,
        }
    }

    pub fn threshold(&self) -> &f32 {
        &self.threshold
    }

    pub fn velocity_scale(&self) -> &f32 {
        &self.velocity_scale
    }
}

impl Default for NoteConfig {
    fn default() -> Self {
        NoteConfig::new(default_threshold(), default_velocity_scale())
    }
}

#[derive(Debug)]
pub struct Note {
    pub note_id: NoteID,
    pub pressed: bool,
    shifted_amount: i8,
    pub velocity: f32,
    pub channel: Channel,
    pub lower_press_time: Option<(Instant, f32)>,
}

impl Note {
    pub fn new(channel: Channel, note: NoteID) -> Note {
        Note {
            note_id: note,
            pressed: false,
            velocity: 0.0,
            shifted_amount: 0,
            channel,
            lower_press_time: None,
        }
    }

    fn get_effective_note(&self) -> Option<NoteID> {
        let computed = self.note_id as i16 + self.shifted_amount as i16;
        if computed >= MIDI_NOTE_MIN.into() && computed <= MIDI_NOTE_MAX.into() {
            Some(computed as NoteID)
        } else {
            None
        }
    }

    fn update_current_value(
        &mut self,
        previous_value: f32,
        new_value: f32,
        sink: &mut impl NoteSink,
        shifted_amount: i8,
        note_config: &NoteConfig,
    ) -> Result<()> {
        // if new_value == 0.0 {
        //     self.velocity = 0.0;
        // } else {
        //     self.velocity = f32::min(
        //         f32::max(
        //             f32::max(0.0, new_value - previous_value) * note_config.velocity_scale()
        //                 - self.velocity,
        //             // This mainly  effects how quickly the velocity decays when there's little movement
        //             self.velocity * 0.90,
        //         ),
        //         1.0,
        //     );
        // }
        // Initialise the
        if (new_value > 0.0 && previous_value == 0.0 && new_value < note_config.threshold)
            || new_value == 0.0
        {
            self.lower_press_time = Some((Instant::now(), new_value));

            self.velocity = 0.0;
        } else if self.lower_press_time.is_some() {
            let (prev_time, prev_depth) = self.lower_press_time.expect("No previous press time");
            let duration = prev_time.elapsed().as_secs_f32();
            // If there's no change there's no velocity
            if new_value == prev_depth {
                self.velocity = 0.0;
            } else {
                self.velocity = f32::min(
                    f32::max(
                        ((new_value - prev_depth) / duration)
                            * (note_config.velocity_scale() / 100.0), // The / 100 is to change the scale of the velocity scale, without it, you have to be working with very small decimal numbers to make noticeable differences in the scale of the velocity
                        0.0,
                    ),
                    1.0,
                );
            }
            // If the value has gone down or there's little difference between the saved previous depth and the new one, so we want to take the time again so the velocity estimate is more accurate
            if (self.lower_press_time.expect("No press time").1 - new_value).abs() < 0.01
                || new_value < previous_value - 0.01
            {
                self.lower_press_time = Some((Instant::now(), new_value));
            }
        }

        // If the modifier pressed state has changed we need to make sure we turn the current note off because the note id will be changed
        if shifted_amount != self.shifted_amount && !self.pressed {
            // if self.pressed {
            //     if let Some(effective_note) = self.get_effective_note() {
            //         sink.note_off(effective_note, self.velocity, self.channel)?;
            //     }
            //     self.pressed = false;
            // }
            self.shifted_amount = shifted_amount;
        }

        if let Some(effective_note) = self.get_effective_note() {
            if new_value > *note_config.threshold() {
                // 'Pressed'
                if !self.pressed {
                    info!(
                        "Triggering with velocity {:?}, prev {:?}, new_val {:?}, elapsed {:?}",
                        self.velocity,
                        self.lower_press_time,
                        new_value,
                        self.lower_press_time.unwrap().0.elapsed()
                    );
                    sink.note_on(effective_note, self.velocity, self.channel)?;
                    self.pressed = true;
                } else {
                    // While we are in the range of what we consider 'pressed' for the key & the note on has already been sent we send aftertouch
                    if AFTERTOUCH && new_value != previous_value {
                        sink.polyphonic_aftertouch(effective_note, new_value, self.channel)?;
                    }
                }
            } else {
                // 'Not Pressed'
                if self.pressed {
                    sink.note_off(effective_note, self.velocity, self.channel)?;
                    self.pressed = false;
                }
            }
        }

        Ok(())
    }

    fn drop(&mut self, sink: &mut Option<impl NoteSink>) -> Result<()> {
        if let Some(sink) = sink {
            if self.pressed {
                if let Some(effective_note) = self.get_effective_note() {
                    sink.note_off(effective_note, self.velocity, self.channel)?;
                }
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
        shifted_amount: i8,
        note_config: &NoteConfig,
    ) -> Result<()> {
        for note in self.notes.iter_mut() {
            note.update_current_value(
                self.current_value,
                new_value,
                sink,
                shifted_amount,
                note_config,
            )?;
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PortOption(usize, String, bool);

pub struct MidiService {
    pub port_options: Option<Vec<PortOption>>,
    connection: Option<MidiOutputConnection>,
    pub keys: HashMap<HIDCodes, Key>,
    pub amount_to_shift: i8,
    pub note_config: NoteConfig,
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
            amount_to_shift: 0,
            note_config: Default::default(),
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

    pub fn set_note_config(&mut self, note_config: NoteConfig) {
        self.note_config = note_config;
    }

    // pub fn init(&mut self, connection_preference: Option<usize>) -> Result<(), Box<dyn Error>> {
    pub fn init(&mut self) -> Result<u32> {
        info!("Starting Wooting Analog SDK!");
        let init_result: SDKResult<u32> = sdk::initialise();
        let device_num = match init_result.0 {
            Ok(device_num) => {
                info!(
                    "Analog SDK Successfully initialised with {} devices",
                    device_num
                );
                let devices: Vec<DeviceInfo> = sdk::get_connected_devices_info(DEVICE_BUFFER_MAX)
                    .0
                    .unwrap();
                assert_eq!(device_num, devices.len() as u32);
                for (i, device) in devices.iter().enumerate() {
                    println!("Device {} is {:?}", i, device);
                }

                device_num
            }
            Err(e) => Err(e).context("Wooting Analog SDK Failed to initialise")?,
        };

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
        Ok(device_num)
    }

    pub fn get_connected_devices(&self) -> Result<Vec<DeviceInfo>> {
        return Ok(sdk::get_connected_devices_info(DEVICE_BUFFER_MAX).0?);
    }

    pub fn select_port(&mut self, option: usize) -> Result<()> {
        //TODO: Deal with the case where the port list has changed since the `port_options` was generated
        if let Some(options) = &self.port_options {
            if option >= options.len() {
                bail!("Port option out of range!");
            }

            let selection = &options[option];

            // Port is already the selected one, don't need to do anything
            if selection.2 {
                return Ok(());
            }

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

            Ok(())
        } else {
            return Err(anyhow!("Port options not initialised"));
        }
    }

    pub fn poll(&mut self) -> Result<()> {
        if self.connection.is_none() {
            bail!("No MIDI connection!");
        }

        let read_result: SDKResult<HashMap<u16, f32>> =
            sdk::read_full_buffer(ANALOG_BUFFER_READ_MAX);
        match read_result.0 {
            Ok(analog_data) => {
                let modifier_pressed = (*analog_data
                    .get(&MODIFIER_KEY.to_u16().unwrap())
                    .unwrap_or(&0.0))
                    >= ACTUATION_POINT;
                for (key_id, key) in self.keys.iter_mut() {
                    let code = key_id.to_u16().expect("Failed to convert HIDCode to u16");
                    let value = analog_data.get(&code).unwrap_or(&0.0);
                    key.update_value(
                        *value,
                        self.connection.as_mut().unwrap(),
                        if modifier_pressed {
                            self.amount_to_shift
                        } else {
                            0
                        },
                        &self.note_config,
                    )?;
                }
            }
            Err(e) => {
                Err(e).context("Failed to read buffer")?;
            }
        };
        Ok(())
    }

    pub fn uninit(&mut self) {
        info!("Uninitialising MidiService");
        sdk::uninitialise();
        trace!("Sdk uninit done");
        if let Some(output) = self.connection.take() {
            output.close();
        }
        trace!("MidiService uninit complete");
    }
}
impl Drop for MidiService {
    fn drop(&mut self) {
        self.uninit();
    }
}
