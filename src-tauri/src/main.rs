#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

extern crate env_logger;
extern crate wooting_analog_midi;
#[macro_use]
extern crate lazy_static;

use log::{error, info, trace, warn};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;
use std::thread::sleep;
use std::time::Duration;
use wooting_analog_midi::{FromPrimitive, HIDCodes, MidiService, Note, REFRESH_RATE};

mod cmd;

use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize)]
struct MidiEntry {
  key: u8,
  note: u8,
  value: f32,
}

#[derive(Serialize)]
struct MidiUpdate {
  data: Vec<MidiEntry>,
}

lazy_static! {
  static ref MIDISERVICE: Arc<RwLock<MidiService>> = Arc::new(RwLock::new(MidiService::new()));
}

fn main() {
  env_logger::try_init();
  // let running = Arc::new(AtomicBool::new(true));

  // let running_inner = running.clone();

  let mut setup = false;

  tauri::AppBuilder::new()
    .setup(move |webview, _source| {
      if !setup {
        let handle = webview.handle();

        thread::spawn(move || {
          MIDISERVICE.write().unwrap().init();
          // while running_inner.load(Ordering::SeqCst) {
          loop {
            match MIDISERVICE.write().unwrap().poll() {
              Ok(_) => {}
              Err(e) => error!("Error: {}", e),
            }

            let notes: &HashMap<HIDCodes, Note> = &MIDISERVICE.read().unwrap().notes;
            let event_message = MidiUpdate {
              data: notes
                .iter()
                .map(|(key, note)| MidiEntry {
                  key: key.clone() as u8,
                  note: note.note_id,
                  value: note.current_value,
                })
                .collect(),
            };
            tauri::event::emit(
              &handle,
              String::from("midi-update"),
              Some(serde_json::to_string(&event_message).unwrap()),
            );
            // let reply = Reply {
            //   data: "something else".to_string(),
            // };

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
    .invoke_handler(|_webview, arg| {
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
          }
          Ok(())
        }
      }
    })
    .build()
    .run();

  // running.store(false, Ordering::SeqCst);
}
