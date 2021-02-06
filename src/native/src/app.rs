#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use crate::settings::AppSettings;
use anyhow::{Context, Result};
#[allow(unused_imports)]
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, RwLock};
use std::thread;
use std::thread::{sleep, JoinHandle};
use std::time::{Duration, Instant};
use wooting_analog_midi::{
  Channel, DeviceInfo, MidiService, NoteID, PortOption, WootingAnalogResult, REFRESH_RATE,
};

// This defines the rate at which midi updates are sent to the UI
pub const MIDI_UPDATE_RATE: u32 = 30; //Hz
const SAVE_THROTTLE: Duration = Duration::from_secs(5);

#[derive(Serialize, Debug)]
struct MidiEntry {
  note: NoteID,
  velocity: f32,
  channel: Channel,
  pressed: bool,
}

#[derive(Serialize, Debug)]
struct MidiUpdateEntry {
  value: f32,
  notes: Vec<MidiEntry>,
}

#[derive(Serialize, Debug)]
pub struct MidiUpdate {
  data: HashMap<u8, MidiUpdateEntry>,
}

#[derive(Deserialize)]
#[serde(tag = "cmd", rename_all = "camelCase")]
pub enum Cmd {
  Function {
    call: AppFunction,
    callback: String,
    error: String,
  },
}

#[derive(Deserialize)]
#[serde(tag = "func", rename_all = "camelCase")]
pub enum AppFunction {
  RequestConfig,
  UpdateConfig { config: String },
  PortOptions,
  SelectPort { option: usize },
}

pub struct App {
  settings: AppSettings,
  thread_pool: Vec<JoinHandle<()>>,
  midi_service: Arc<RwLock<MidiService>>,
  running: Arc<AtomicBool>,
  last_save: Option<Instant>,
}

impl App {
  pub fn new() -> Self {
    App {
      settings: AppSettings::default(),
      thread_pool: vec![],
      midi_service: Arc::new(RwLock::new(MidiService::new())),
      running: Arc::new(AtomicBool::new(true)),
      last_save: None,
    }
  }

  pub fn init(&mut self) -> Result<Receiver<AppEvent>> {
    self.settings = AppSettings::load_config().context("Failed to load App Settings")?;
    {
      let mut midi = self.midi_service.write().unwrap();
      midi
        .update_mapping(&self.settings.get_proper_mapping())
        .with_context(|| "Failed to initialise loaded mapping")?;
      midi.amount_to_shift = self.settings.shift_amount;
    }

    let device_count = self.midi_service.write().unwrap().init()?;

    let mut has_devices: bool = device_count > 0;

    let (tx, rx) = channel::<AppEvent>();

    let running_inner = self.running.clone();
    let midi_service_inner = self.midi_service.clone();
    let tx_inner = tx.clone();
    self.thread_pool.push(thread::spawn(move || {
      let mut iter_count: u32 = 0;
      if has_devices {
        let devices = midi_service_inner
          .read()
          .unwrap()
          .get_connected_devices()
          .context("Failed to get connected devices")
          .map_err(output_err)
          .unwrap_or(vec![]);
        if let Err(e) = tx_inner
          .send(AppEvent::FoundDevices(devices))
          .context("Error when sending FoundDevices event!")
        {
          output_err(e);
        }
      }

      if let Err(e) = tx_inner
        .send(AppEvent::PortOptions(
          midi_service_inner
            .read()
            .unwrap()
            .port_options
            .as_ref()
            .cloned()
            .expect("There should be at least some port options"),
        ))
        .context("Error when sending FoundDevices event!")
      {
        output_err(e);
      }

      while running_inner.load(Ordering::SeqCst) {
        let mut errored = false;
        // We have to do this hacky structure to ensure the write lock gets dropped before the read lock later on
        {
          let result = midi_service_inner.write().unwrap().poll();
          if let Err(e) = result
          // .map_err(output_err)
          {
            errored = true;
            match e.root_cause().downcast_ref::<WootingAnalogResult>() {
              Some(WootingAnalogResult::NoDevices) => {
                if has_devices {
                  has_devices = false;
                  warn!("{}", WootingAnalogResult::NoDevices);
                  if let Err(e) = tx_inner
                    .send(AppEvent::NoDevices)
                    .context("Error when sending NoDevices event!")
                  {
                    output_err(e);
                  }
                }
              }
              Some(_) | None => {
                error!("{}", e);
              }
            };
          }
        }

        if !errored {
          if !has_devices {
            let devices = midi_service_inner
              .read()
              .unwrap()
              .get_connected_devices()
              .context("Failed to get connected devices")
              .map_err(output_err)
              .unwrap_or(vec![]);
            if let Err(e) = tx_inner
              .send(AppEvent::FoundDevices(devices))
              .context("Error when sending FoundDevices event!")
            {
              output_err(e);
            } else {
              has_devices = true;
            }
          }

          if (iter_count % (REFRESH_RATE as u32 / MIDI_UPDATE_RATE)) == 0 {
            let keys = &midi_service_inner.read().unwrap().keys;
            let event_message = MidiUpdate {
              data: keys
                .iter()
                .filter_map(|(key_id, key)| {
                  if key.notes.len() > 0 || key.current_value > 0.0 {
                    Some((
                      key_id.clone() as u8,
                      MidiUpdateEntry {
                        value: key.current_value,
                        notes: key
                          .notes
                          .iter()
                          .map(|note| MidiEntry {
                            note: note.note_id,
                            velocity: note.velocity,
                            channel: note.channel,
                            pressed: note.pressed,
                          })
                          .collect(),
                      },
                    ))
                  } else {
                    None
                  }
                })
                .collect(),
            };
            if let Err(e) = tx_inner.send(AppEvent::MidiUpdate(event_message)) {
              error!("Error while sending App Update, {:#?}", e);
            }
          }
        }

        iter_count += 1;
        sleep(Duration::from_secs_f32(1.0 / REFRESH_RATE))
      }
    }));

    // self.event_sender = Some(tx);

    Ok(rx)
  }

  fn update_config(&mut self, config: AppSettings) {
    self.settings = config;
    {
      let mut midi = self.midi_service.write().unwrap();
      //Update the service with the new mapping
      if let Err(e) = midi.update_mapping(&self.settings.get_proper_mapping()) {
        error!("Error updating midi service mapping! {:#?}", e);
      }
      midi.amount_to_shift = self.settings.shift_amount;
      midi.set_note_config(self.settings.note_config.clone());
    }
    self.save_config();
  }

  fn save_config(&mut self) {
    if self.last_save.is_none() || self.last_save.unwrap().elapsed() >= SAVE_THROTTLE {
      if let Err(e) = self.settings.save_config() {
        error!("Error saving: {:#?}", e);
      } else {
        self.last_save = Some(Instant::now());
      }
    }
  }

  fn get_config_string(&self) -> Value {
    serde_json::to_value(&self.settings).unwrap()
  }

  fn get_port_options_string(&self) -> Value {
    serde_json::to_value(&self.midi_service.read().unwrap().port_options).unwrap()
  }

  pub fn exec_loop<F: 'static>(&mut self, mut f: F)
  where
    F: FnMut() + Send,
  {
    let running_inner = self.running.clone();
    self.thread_pool.push(thread::spawn(move || {
      while running_inner.load(Ordering::SeqCst) {
        f();
      }
    }));
  }

  fn select_port(&mut self, option: usize) -> Result<Value> {
    self.midi_service.write().unwrap().select_port(option)?;
    Ok(serde_json::to_value(
      self
        .midi_service
        .read()
        .unwrap()
        .port_options
        .as_ref()
        .unwrap(),
    )?)
  }

  fn uninit(&mut self) {
    if let Err(e) = self.settings.save_config() {
      error!("Error saving config! {}", e);
    }

    self.running.store(false, Ordering::SeqCst);
    for thread in self.thread_pool.drain(..) {
      if let Err(e) = thread.join() {
        error!("Error joining thread: {:?}", e);
      }
    }
    self.midi_service.write().unwrap().uninit();
  }

  pub fn process_command(&mut self, command: AppFunction) -> Result<Value> {
    match command {
      AppFunction::RequestConfig => Ok(self.get_config_string()),
      AppFunction::UpdateConfig { config } => {
        let config = serde_json::from_str(&config[..]).unwrap();

        self.update_config(config);
        Ok(Value::Null)
      }
      AppFunction::PortOptions => Ok(self.get_port_options_string()),
      AppFunction::SelectPort { option } => Ok(self.select_port(option)?),
    }
  }
}

impl Drop for App {
  fn drop(&mut self) {
    self.uninit();
  }
}

// fn output_err<T: std::fmt::Display>(error: T) -> T {
//   error!("Error: {:#?}", error);
//   error
// }
fn output_err(error: anyhow::Error) -> anyhow::Error {
  error!("Error: {:#?}", error);
  error
}

pub enum ChannelMessage {
  UpdateBindings,
}

#[derive(Serialize, Debug)]
#[serde(tag = "type", content = "value")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AppEvent {
  MidiUpdate(MidiUpdate),
  NoDevices,
  FoundDevices(Vec<DeviceInfo>),
  PortOptions(Vec<PortOption>),
}

// fn emit_event(handle: &mut tauri::WebviewMut, event_name: &str, param: Option<String>) {
//   if let Err(e) = tauri::event::emit(handle, String::from(event_name), param) {
//     error!("Error emitting event! {}", e);
//   }
// }

// fn main() {
//   if let Err(e) = env_logger::try_init() {
//     warn!("Failed to init env_logger, {}", e);
//   }

//   let mut setup = false;
//   tauri::AppBuilder::new()
//     .setup(move |webview, _source| {
//       // if setup {
//       //   APP.write().unwrap().uninit();
//       //   setup = false;
//       // }

//       if !setup {
//         setup = true;
//         let mut handle = webview.as_mut();
//         let event_receiver = {
//           match APP.write().unwrap().init() {
//             Ok(recv) => recv,
//             Err(e) => {
//               let message = format!("\"{}\".\n\nPlease make sure you have all the dependencies installed correctly including the Analog SDK!", e);
//               error!("{}", message);
//               tauri::api::dialog::message(message, "Fatal error occured on initialisation!");
//               panic!(format!("{}", e));
//             }
//           }
//         };

//         APP.write().unwrap().exec_loop(move || {
//           if let Ok(event) = event_receiver
//             .recv()
//             // .map_err(|err| warn!("Error on event reciever: {}", err))
//           {
//             emit_event(&mut handle, "event", Some(serde_json::to_string(&event).expect("Failed to serialize event")));

//             // match event {
//             //   AppEvent::MidiUpdate(update) => {
//             //     emit_event(&mut handle, "midi-update", Some(serde_json::to_string(&update).unwrap()));

//             //   },
//             //   AppEvent::FoundDevices(devices) => {
//             //     emit_event(&mut handle, "found-devices", Some(serde_json::to_string(&devices).unwrap()));
//             //   },
//             //   AppEvent::NoDevices => {
//             //     emit_event(&mut handle, "no-devices", None);
//             //   }
//             // }
//           }
//         })
//       }
//       let mut handle = webview.as_mut();
//       emit_event(&mut handle, "init-complete", None);
//     })
//     .invoke_handler(move |_webview, arg| match serde_json::from_str(arg) {
//       Err(e) => Err(e.to_string()),
//       Ok(command) => {
//         match command {
//           Cmd::Function {
//             call,
//             callback,
//             error,
//           } => {
//             tauri::execute_promise(
//               _webview,
//               move || APP.write().unwrap().process_command(call),
//               callback,
//               error,
//             );
//           }
//         }
//         Ok(())
//       }
//     })
//     .build()
//     .run();
//   println!("After run");
//   APP.write().unwrap().uninit();
// }
