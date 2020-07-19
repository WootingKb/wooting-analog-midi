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
#[allow(unused_imports)]
#[macro_use]
extern crate anyhow;

#[allow(unused_imports)]
use log::{error, info, warn};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;
use std::thread::{sleep, JoinHandle};
use std::time::Duration;
use wooting_analog_midi::{Channel, MidiService, NoteID, REFRESH_RATE};
mod settings;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use settings::AppSettings;
use std::collections::HashMap;
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;

#[derive(Serialize)]
struct MidiEntry {
  note: NoteID,
  velocity: f32,
  channel: Channel,
  pressed: bool,
}

#[derive(Serialize)]
struct MidiUpdateEntry {
  value: f32,
  notes: Vec<MidiEntry>,
}

#[derive(Serialize)]
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
              error!("Error while sending App Update, {}", e);
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

  fn get_config_string(&self) -> Value {
    serde_json::to_value(&self.settings).unwrap()
  }

  fn get_port_options_string(&self) -> Value {
    serde_json::to_value(&self.midi_service.read().unwrap().port_options).unwrap()
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

  fn process_command(&mut self, command: AppFunction) -> Result<Value> {
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

fn output_err<T: std::fmt::Display>(error: T) -> T {
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
    .invoke_handler(move |_webview, arg| match serde_json::from_str(arg) {
      Err(e) => Err(e.to_string()),
      Ok(command) => {
        match command {
          Cmd::Function {
            call,
            callback,
            error,
          } => {
            tauri::execute_promise(
              _webview,
              move || APP.write().unwrap().process_command(call),
              callback,
              error,
            );
          }
        }
        Ok(())
      }
    })
    .build()
    .run();
  println!("After run");
  APP.write().unwrap().uninit();
}
