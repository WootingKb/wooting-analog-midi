#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

extern crate env_logger;
extern crate wooting_analog_midi;
#[macro_use]
extern crate lazy_static;

use log::{error, info, trace, warn};
use std::error::Error;
use std::fs::OpenOptions;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;
use std::thread::sleep;
use std::time::Duration;
use tauri::api::path::config_dir;
use wooting_analog_midi::{FromPrimitive, HIDCodes, MidiService, Note, REFRESH_RATE};
mod cmd;
use cmd::AppSettings;
use std::fs::create_dir;
use std::io::{Read, Write};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize)]
struct MidiEntry {
  key: u8,
  note: Option<u8>,
  value: f32,
}

#[derive(Serialize)]
struct MidiUpdate {
  data: Vec<MidiEntry>,
}

lazy_static! {
  static ref MIDISERVICE: Arc<RwLock<MidiService>> = Arc::new(RwLock::new(MidiService::new()));
}

fn default_mapping() -> AppSettings {
  AppSettings {
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
const CONFIG_DIR: &str = "wooting-midi";
const CONFIG_FILE: &str = "config.json";

fn config_path() -> Result<PathBuf, Box<dyn Error>> {
  let mut config_file = config_dir().ok_or("No config dir!")?;
  config_file.push(CONFIG_DIR);
  if !config_file.exists() {
    create_dir(&config_file)?;
  }

  config_file.push(CONFIG_FILE);
  Ok(config_file)
}

fn load_config() -> Result<AppSettings, Box<dyn Error>> {
  let config_file = config_path()?;

  let mut file = OpenOptions::new()
    .read(true)
    .write(true)
    .create(true)
    .open(config_file)?;
  let mut content: String = String::new();
  let size = file.read_to_string(&mut content)?;

  if content.is_empty() {
    let default = default_mapping();
    file.write_all(&serde_json::to_vec(&default)?[..])?;
    Ok(default)
  } else {
    Ok(serde_json::from_str::<AppSettings>(&content[..])?)
  }
}

fn save_config(settings: &AppSettings) -> Result<(), Box<dyn Error>> {
  let config_file = config_path()?;
  info!("Saving to {:?}", config_file);

  let mut file = OpenOptions::new()
    .read(true)
    .write(true)
    .create(true)
    .open(config_file)?;

  file.write_all(&serde_json::to_vec(&settings)?[..])?;
  Ok(())
}

fn get_proper_mapping(mapping: &HashMap<u8, u8>) -> HashMap<HIDCodes, u8> {
  mapping
    .iter()
    .map(|(key, note)| (HIDCodes::from_u8(*key).unwrap(), *note))
    .collect()
}
pub const MIDI_UPDATE_RATE: u32 = 15; //Hz

fn main() {
  if let Err(e) = env_logger::try_init() {
    warn!("Failed to init env_logger, {}", e);
  }
  let mut config = Arc::new(RwLock::new(load_config().unwrap()));
  // let running = Arc::new(AtomicBool::new(true));
  let config_changed = Arc::new(RwLock::new(false));

  // let running_inner = running.clone();

  let mut setup = false;
  let config_inner = config.clone();
  let config_changed_inner = config_changed.clone();
  let config_changed_inner1 = config_changed.clone();
  let config_inner1 = config.clone();

  tauri::AppBuilder::new()
    .setup(move |webview, _source| {
      if !setup {
        let handle = webview.handle();
        let config_inner1 = config_inner1.clone();
        let config_changed_inner1 = config_changed_inner1.clone();

        thread::spawn(move || {
          let initial_mapping = get_proper_mapping(&config_inner1.read().unwrap().keymapping);
          MIDISERVICE
            .write()
            .unwrap()
            .update_mapping(&initial_mapping);

          match MIDISERVICE.write().unwrap().init() {
            Ok(_) => {}
            Err(e) => error!("Error: {}", e),
          }
          // while running_inner.load(Ordering::SeqCst) {
          let mut iter_count: u32 = 0;
          loop {
            if *config_changed_inner1.read().unwrap() {
              *config_changed_inner1.write().unwrap() = false;
              //Update mapping
              let mapping = get_proper_mapping(&config_inner1.read().unwrap().keymapping);
              MIDISERVICE.write().unwrap().update_mapping(&mapping);
            }

            match MIDISERVICE.write().unwrap().poll() {
              Ok(_) => {}
              Err(e) => error!("Error: {}", e),
            }
            if (iter_count % (REFRESH_RATE as u32 / MIDI_UPDATE_RATE)) == 0 {
              let notes: &HashMap<HIDCodes, Note> = &MIDISERVICE.read().unwrap().notes;
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
              tauri::event::emit(
                &handle,
                String::from("midi-update"),
                Some(serde_json::to_string(&event_message).unwrap()),
              );
            }
            // let reply = Reply {
            //   data: "something else".to_string(),
            // };

            iter_count += 1;
            sleep(Duration::from_secs_f32(1.0 / REFRESH_RATE))
          }
        });
      }
      // tauri::event::listen(String::from("js-event"), move |msg| {
      //   println!("got js-event with message '{:?}'", msg);
      //   let reply = Reply {
      //     data: "something else".to_string(),
      //   };

      //   tauri::event::emit(
      //     &handle,
      //     String::from("rust-event"),
      //     Some(serde_json::to_string(&reply).unwrap()),
      //   );
      // });

      // webview.eval("window.onTauriInit()").unwrap();
    })
    .invoke_handler(move |_webview, arg| {
      let config_inner1 = config_inner.clone();
      let config_inner2 = config_inner.clone();
      use cmd::Cmd::*;
      match serde_json::from_str(arg) {
        Err(e) => Err(e.to_string()),
        Ok(command) => {
          match command {
            LogOperation { event, payload } => {
              println!("{} {:?}", event, payload);
            }
            PerformRequest {
              endpoint,
              body,
              callback,
              error,
            } => {
              // tauri::execute_promise is a helper for APIs that uses the tauri.promisified JS function
              // so you can easily communicate between JS and Rust with promises
              tauri::execute_promise(
                _webview,
                move || {
                  println!("{} {:?}", endpoint, body);
                  // perform an async operation here
                  // if the returned value is Ok, the promise will be resolved with its value
                  // if the returned value is Err, the promise will be rejected with its value
                  // the value is a string that will be eval'd
                  Ok("{ key: 'response', value: [{ id: 3 }] }".to_string())
                },
                callback,
                error,
              )
            }
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
                  Ok(serde_json::to_string(&(*config_inner1.read().unwrap())).unwrap())
                },
                callback,
                error,
              )
            }
            UpdateConfig { config } => {
              let config = serde_json::from_str(&config[..]).unwrap();

              *config_inner2.write().unwrap() = config;
              *config_changed_inner.write().unwrap() = true;
              if let Err(e) = save_config(&(*config_inner2.read().unwrap())) {
                error!("Error saving: {}", e);
              }
            }
          }
          Ok(())
        }
      }
    })
    .build()
    .run();
  save_config(&config.read().unwrap()).unwrap();
  // running.store(false, Ordering::SeqCst);
}
