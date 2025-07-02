#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

extern crate env_logger;
extern crate wooting_analog_midi_core;
#[macro_use]
extern crate lazy_static;

#[allow(unused_imports)]
#[macro_use]
extern crate anyhow;

use log::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;
use std::thread::{sleep, JoinHandle};
use wooting_analog_midi_core::{
  Channel, DeviceInfo, MidiService, NoteID, PortOption, WootingAnalogResult, REFRESH_RATE,
};
mod settings;
use anyhow::{Context, Result};
use flume::Receiver;
use serde::Serialize;
use settings::AppSettings;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tauri::{Manager, Menu, MenuItem, Submenu};

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

struct App {
  settings: AppSettings,
  thread_pool: Vec<JoinHandle<()>>,
  midi_service: Arc<RwLock<MidiService>>,
  running: Arc<AtomicBool>,
  last_save: Option<Instant>,
  event_receiver: Option<flume::Receiver<AppEvent>>,
}

impl App {
  fn new() -> Self {
    App {
      settings: AppSettings::default(),
      thread_pool: vec![],
      midi_service: Arc::new(RwLock::new(MidiService::new())),
      running: Arc::new(AtomicBool::new(true)),
      last_save: None,
      event_receiver: None,
    }
  }

  fn init(&mut self) -> Result<()> {
    self.settings = AppSettings::load_config().context("Failed to load App Settings")?;
    {
      let mut midi = self.midi_service.write().unwrap();
      midi
        .update_mapping(&self.settings.get_proper_mapping())
        .with_context(|| "Failed to initialise loaded mapping")?;
      midi.amount_to_shift = self.settings.shift_amount;
    }

    let mut device_count = self.midi_service.write().unwrap().init()? as usize;

    let (tx, rx) = flume::unbounded::<AppEvent>();

    let running_inner = self.running.clone();
    let midi_service_inner = self.midi_service.clone();
    let tx_inner = tx.clone();

    self.thread_pool.push(thread::spawn(move || {
      let mut iter_count: u32 = 0;
      // if has_devices {
      //   let devices = midi_service_inner
      //     .read()
      //     .unwrap()
      //     .get_connected_devices()
      //     .context("Failed to get connected devices")
      //     .map_err(output_err)
      //     .unwrap_or(vec![]);
      //   if let Err(e) = tx_inner
      //     .send(AppEvent::FoundDevices(devices))
      //     .context("Error when sending FoundDevices event!")
      //   {
      //     output_err(e);
      //   }
      // }

      // if let Err(e) = tx_inner
      //   .send(AppEvent::PortOptions(
      //     midi_service_inner
      //       .read()
      //       .unwrap()
      //       .port_options
      //       .as_ref()
      //       .cloned()
      //       .expect("There should be at least some port options"),
      //   ))
      //   .context("Error when sending FoundDevices event!")
      // {
      //   output_err(e);
      // }

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
                if device_count != 0 {
                  device_count = 0;
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
          // This should be replaced with a handler for the device disconnected/connected events
          let devices = midi_service_inner
            .read()
            .unwrap()
            .get_connected_devices()
            .context("Failed to get connected devices")
            .map_err(output_err)
            .unwrap_or(vec![]);
          let devices_len = devices.len();
          if device_count != devices_len {
            if let Err(e) = tx_inner
              .send(AppEvent::FoundDevices(devices))
              .context("Error when sending FoundDevices event!")
            {
              output_err(e);
            } else {
              device_count = devices_len;
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

    self.event_receiver = Some(rx);
    Ok(())
  }

  fn listen(&mut self) -> Result<Receiver<AppEvent>> {
    self
      .event_receiver
      .clone()
      .ok_or_else(|| anyhow!("Failed to retrieve event listener"))
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

  fn get_port_options(&self) -> Vec<PortOption> {
    self
      .midi_service
      .read()
      .unwrap()
      .port_options
      .as_ref()
      .cloned()
      .unwrap_or(vec![])
  }

  fn get_connected_devices(&self) -> Vec<DeviceInfo> {
    self
      .midi_service
      .read()
      .unwrap()
      .get_connected_devices()
      .context("Failed to get connected devices")
      .map_err(output_err)
      .unwrap_or(vec![])
  }

  fn exec_loop<F: 'static>(&mut self, mut f: F)
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

  fn select_port(&mut self, option: usize) -> Result<Vec<PortOption>> {
    self.midi_service.write().unwrap().select_port(option)?;
    Ok(
      self
        .midi_service
        .read()
        .unwrap()
        .port_options
        .as_ref()
        .cloned()
        .unwrap_or(vec![]),
    )
  }

  fn uninit(&mut self) {
    if let Err(e) = self.settings.save_config() {
      error!("Error saving config! {}", e);
    }

    trace!("waiting for thread");
    self.running.store(false, Ordering::SeqCst);
    for thread in self.thread_pool.drain(..) {
      if let Err(e) = thread.join() {
        error!("Error joining thread: {:?}", e);
      }
    }
    trace!("thread wait done");
    self.midi_service.write().unwrap().uninit();
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

#[derive(Serialize, Debug)]
#[serde(tag = "type", content = "value")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AppEvent {
  MidiUpdate(MidiUpdate),
  NoDevices,
  FoundDevices(Vec<DeviceInfo>),
  PortOptions(Vec<PortOption>),
}

lazy_static! {
  static ref APP: RwLock<App> = RwLock::new(App::new());
}

#[derive(Debug, Clone, Serialize)]
struct CommandError {
  message: String,
}

impl CommandError {
  fn new(message: String) -> Self {
    Self { message }
  }
}

impl std::fmt::Display for CommandError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.message)
  }
}

impl std::error::Error for CommandError {}

impl From<anyhow::Error> for CommandError {
  fn from(err: anyhow::Error) -> CommandError {
    CommandError::new(format!("{:#?}", err))
  }
}

#[tauri::command]
fn get_config() -> AppSettings {
  APP.read().unwrap().settings.clone()
}

#[tauri::command]
fn update_config(config: AppSettings) {
  APP.write().unwrap().update_config(config);
}

#[tauri::command]
fn get_port_options() -> Vec<PortOption> {
  APP.write().unwrap().get_port_options()
}

#[tauri::command]
fn get_connected_devices() -> Vec<DeviceInfo> {
  APP.write().unwrap().get_connected_devices()
}

#[tauri::command]
fn select_port(option: usize) -> Result<Vec<PortOption>, CommandError> {
  Ok(APP.write().unwrap().select_port(option)?)
}

fn main_menu() -> Menu {
  let app_menu = Menu::new()
    .add_native_item(MenuItem::Hide)
    .add_native_item(MenuItem::HideOthers)
    .add_native_item(MenuItem::ShowAll)
    .add_native_item(MenuItem::Separator)
    .add_native_item(MenuItem::Quit);
  let edit_menu = Menu::new()
    .add_native_item(MenuItem::Undo)
    .add_native_item(MenuItem::Copy)
    .add_native_item(MenuItem::Cut)
    .add_native_item(MenuItem::Paste)
    .add_native_item(MenuItem::Separator)
    .add_native_item(MenuItem::Redo)
    .add_native_item(MenuItem::SelectAll);
  Menu::new()
    .add_submenu(Submenu::new("Wooting Analog Midi", app_menu))
    .add_submenu(Submenu::new("Edit", edit_menu))
}

fn main() -> Result<()> {
  if let Err(e) = env_logger::try_init() {
    warn!("Failed to init env_logger, {}", e);
  }

  if let Err(e) = APP.write().unwrap().init() {
    let message = format!("{}.\n\nPlease make sure you have all the dependencies installed correctly including the Analog SDK!", e);
    error!("{}", message);

    msgbox::create("Error Occured", &message, msgbox::IconType::Error)?;

    panic!("{}", e);
  }

  tauri::Builder::default()
    .menu(main_menu())
    .invoke_handler(tauri::generate_handler![
      get_config,
      update_config,
      get_port_options,
      select_port,
      get_connected_devices
    ])
    .setup(|app| {
      #[cfg(debug_assertions)]
      app.get_window("main").unwrap().open_devtools();

      Ok(())
    })
    .on_page_load(move |window, _payload| {
      let event_receiver = APP
        .write()
        .unwrap()
        .listen()
        .expect("Failed to listen to app events");

      let window_inner = window.clone();
      APP.write().unwrap().exec_loop(move || {
        if let Ok(event) = event_receiver.recv() {
          window_inner
            .emit(
              "event",
              Some(serde_json::to_string(&event).expect("Failed to serialize event")),
            )
            .expect("Failed to emit event");
        }
      });
    })
    .run(tauri::generate_context!())
    .unwrap();

  trace!("After run");
  APP.write().unwrap().uninit();
  trace!("Uninit");
  Ok(())
}
