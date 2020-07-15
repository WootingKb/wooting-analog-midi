#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

extern crate env_logger;
extern crate wooting_analog_midi;
#[macro_use]
extern crate lazy_static;
// #[macro_use]
// extern crate crossbeam_channel;
#[macro_use]
extern crate anyhow;

use log::{error, info, warn};
use std::error::Error;
use std::fs::OpenOptions;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;
use std::thread::{sleep, JoinHandle};
use std::time::Duration;
use tauri::api::path::config_dir;
use wooting_analog_midi::{FromPrimitive, HIDCodes, MidiService, Note, PortOption, REFRESH_RATE};
mod cmd;
use anyhow::{Context, Result};
use cmd::AppSettings;
use serde::Serialize;
use std::collections::HashMap;
use std::fs::create_dir;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::sync::mpsc::{Receiver, Sender};

#[derive(Serialize)]
struct MidiEntry {
  key: u8,
  note: Option<u8>,
  value: f32,
}

#[derive(Serialize)]
pub struct MidiUpdate {
  data: Vec<MidiEntry>,
}

const CONFIG_DIR: &str = "wooting-midi";
const CONFIG_FILE: &str = "config.json";
impl AppSettings {
  fn config_path() -> Result<PathBuf> {
    let mut config_file = config_dir().context("No config dir!")?;
    config_file.push(CONFIG_DIR);
    if !config_file.exists() {
      create_dir(&config_file)?;
    }
    config_file.push(CONFIG_FILE);
    Ok(config_file)
  }

  fn load_config() -> Result<AppSettings> {
    let config_file = Self::config_path()?;
    let mut file = OpenOptions::new()
      .read(true)
      .write(true)
      .create(true)
      .open(config_file)?;
    let mut content: String = String::new();
    let size = file.read_to_string(&mut content)?;
    if content.is_empty() {
      let default = Self::default();
      file.write_all(&serde_json::to_vec(&default)?[..])?;
      Ok(default)
    } else {
      Ok(serde_json::from_str::<AppSettings>(&content.trim()[..])?)
    }
  }

  fn save_config(&self) -> Result<()> {
    let config_file = Self::config_path()?;
    info!("Saving to {:?}", config_file);
    let mut file = OpenOptions::new()
      .read(true)
      .write(true)
      .create(true)
      .truncate(true)
      .open(config_file)?;
    file.write_all(&serde_json::to_vec(&self)?[..])?;
    Ok(())
  }

  pub fn get_proper_mapping(&self) -> HashMap<HIDCodes, u8> {
    self
      .keymapping
      .iter()
      .map(|(key, note)| (HIDCodes::from_u8(*key).unwrap(), *note))
      .collect()
  }
}

impl Default for AppSettings {
  fn default() -> Self {
    Self {
      keymapping: [
        (HIDCodes::Q as u8, 57),
        (HIDCodes::W as u8, 58),
        (HIDCodes::E as u8, 59),
        (HIDCodes::R as u8, 60),
        (HIDCodes::T as u8, 61),
        (HIDCodes::Y as u8, 62),
        (HIDCodes::U as u8, 63),
        (HIDCodes::I as u8, 64),
        (HIDCodes::O as u8, 65),
        (HIDCodes::P as u8, 66),
      ]
      .iter()
      .cloned()
      .collect(),
    }
  }
}

struct App {
  settings: AppSettings,
  thread_pool: Vec<JoinHandle<()>>,
  midi_service: Arc<RwLock<MidiService>>,
  running: Arc<AtomicBool>,
  // event_sender: Option<Sender<AppEvent>>,
}

impl App {
  fn new() -> Self {
    App {
      settings: AppSettings::default(),
      thread_pool: vec![],
      midi_service: Arc::new(RwLock::new(MidiService::new())),
      running: Arc::new(AtomicBool::new(true)),
      // event_sender: None,
    }
  }

  fn init(&mut self) -> Result<Receiver<AppEvent>> {
    self.settings = AppSettings::load_config().context("Failed to load App Settings")?;

    self
      .midi_service
      .write()
      .unwrap()
      .update_mapping(&self.settings.get_proper_mapping())
      .with_context(|| "Failed to initialise loaded mapping")?;

    self.midi_service.write().unwrap().init()?;

    let (tx, rx) = channel::<AppEvent>();

    let running_inner = self.running.clone();
    let midi_service_inner = self.midi_service.clone();
    let tx_inner = tx.clone();
    self.thread_pool.push(thread::spawn(move || {
      let mut iter_count: u32 = 0;
      while running_inner.load(Ordering::SeqCst) {
        if midi_service_inner
          .write()
          .unwrap()
          .poll()
          .map_err(output_err)
          .is_ok()
        {
          if (iter_count % (REFRESH_RATE as u32 / MIDI_UPDATE_RATE)) == 0 {
            let notes: &HashMap<HIDCodes, Note> = &midi_service_inner.read().unwrap().notes;
            let event_message = MidiUpdate {
              data: notes
                .iter()
                .filter_map(|(key, note)| {
                  if note.note_id.is_some() || note.current_value > 0.0 {
                    Some(MidiEntry {
                      key: key.clone() as u8,
                      note: note.note_id,
                      value: note.current_value,
                    })
                  } else {
                    None
                  }
                })
                .collect(),
            };
            tx_inner.send(AppEvent::MidiUpdate(event_message));
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
    //Update the service with the new mapping
    if let Err(e) = self
      .midi_service
      .write()
      .unwrap()
      .update_mapping(&self.settings.get_proper_mapping())
    {
      error!("Error updating midi service mapping! {}", e);
    }

    if let Err(e) = self.settings.save_config() {
      error!("Error saving: {}", e);
    }
  }

  fn get_config_string(&self) -> String {
    serde_json::to_string(&self.settings).unwrap()
  }

  fn get_port_options_string(&self) -> String {
    serde_json::to_string(&self.midi_service.read().unwrap().port_options).unwrap()
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

  fn select_port(&mut self, option: usize) -> Result<String> {
    self.midi_service.write().unwrap().select_port(option)?;
    Ok(serde_json::to_string(
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
}

impl Drop for App {
  fn drop(&mut self) {
    self.uninit();
  }
}

fn output_err(error: Box<dyn Error>) -> Box<dyn Error> {
  error!("Error: {}", error);
  error
}

pub enum ChannelMessage {
  UpdateBindings,
}
pub enum AppEvent {
  MidiUpdate(MidiUpdate),
  // InitComplete,
}

lazy_static! {
  static ref APP: Arc<RwLock<App>> = Arc::new(RwLock::new(App::new()));
}

pub const MIDI_UPDATE_RATE: u32 = 15; //Hz

fn main() {
  if let Err(e) = env_logger::try_init() {
    warn!("Failed to init env_logger, {}", e);
  }

  let mut setup = false;
  tauri::AppBuilder::new()
    .setup(move |webview, _source| {
      if !setup {
        setup = true;
        let handle = webview.handle();
        let event_receiver = APP.write().unwrap().init().unwrap();

        APP.write().unwrap().exec_loop(move || {
          if let Ok(event) = event_receiver
            .recv()
            .map_err(|err| warn!("Error on event reciever: {}", err))
          {
            match event {
              AppEvent::MidiUpdate(update) => {
                if let Err(e) = tauri::event::emit(
                  &handle,
                  String::from("midi-update"),
                  Some(serde_json::to_string(&update).unwrap()),
                ) {
                  error!("Error emitting event! {}", e);
                }
              }
            }
          }
        })
      }
      let handle = webview.handle();
      if let Err(e) = tauri::event::emit(&handle, String::from("init-complete"), Option::<()>::None)
      {
        error!("Error emitting event! {}", e);
      }
    })
    .invoke_handler(move |_webview, arg| {
      use cmd::Cmd::*;
      match serde_json::from_str(arg) {
        Err(e) => Err(e.to_string()),
        Ok(command) => {
          match command {
            RequestConfig { callback, error } => {
              // tauri::execute_promise is a helper for APIs that uses the tauri.promisified JS function
              // so you can easily communicate between JS and Rust with promises
              tauri::execute_promise(
                _webview,
                move || {
                  // println!("{} {:?}", endpoint, body);
                  // perform an async operation here
                  // if the returned value is Ok, the promise will be resolved with its value
                  // if the returned value is Err, the promise will be rejected with its value
                  // the value is a string that will be eval'd
                  Ok(APP.read().unwrap().get_config_string())
                },
                callback,
                error,
              )
            }
            UpdateConfig { config } => {
              let config = serde_json::from_str(&config[..]).unwrap();

              APP.write().unwrap().update_config(config);
            }
            PortOptions { callback, error } => {
              // tauri::execute_promise is a helper for APIs that uses the tauri.promisified JS function
              // so you can easily communicate between JS and Rust with promises
              tauri::execute_promise(
                _webview,
                move || {
                  // println!("{} {:?}", endpoint, body);
                  // perform an async operation here
                  // if the returned value is Ok, the promise will be resolved with its value
                  // if the returned value is Err, the promise will be rejected with its value
                  // the value is a string that will be eval'd
                  Ok(APP.read().unwrap().get_port_options_string())
                },
                callback,
                error,
              )
            }
            SelectPort {
              option,
              callback,
              error,
            } => tauri::execute_promise(
              _webview,
              move || {
                APP
                  .write()
                  .unwrap()
                  .select_port(option)
                  .map(|data| serde_json::to_string(&data).unwrap())
              },
              callback,
              error,
            ),
          }
          Ok(())
        }
      }
    })
    .build()
    .run();
  println!("After run");
  APP.write().unwrap().uninit();
}
