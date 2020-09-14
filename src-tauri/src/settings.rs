use anyhow::{Context, Result};
#[allow(unused_imports)]
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::create_dir;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::PathBuf;
use tauri::api::path::config_dir;
use wooting_analog_midi::{Channel, FromPrimitive, HIDCodes, NoteID};

fn default_shift_amount() -> u8 {
  12
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AppSettings {
  //Channel -> [(key, note)]
  pub keymapping: HashMap<Channel, Vec<(u8, NoteID)>>,
  #[serde(default = "default_shift_amount")]
  pub shift_amount: u8,
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

  pub fn load_config() -> Result<AppSettings> {
    let config_file = Self::config_path()?;
    let mut file = OpenOptions::new()
      .read(true)
      .write(true)
      .create(true)
      .open(config_file)?;
    let mut content: String = String::new();
    let _size = file.read_to_string(&mut content)?;
    if content.is_empty() {
      let default = Self::default();
      file.write_all(&serde_json::to_vec(&default)?[..])?;
      Ok(default)
    } else {
      Ok(serde_json::from_str::<AppSettings>(&content.trim()[..])?)
    }
  }

  pub fn save_config(&self) -> Result<()> {
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
  // Channel -> [(key, note)] => key -> [(channel, note)]
  pub fn get_proper_mapping(&self) -> HashMap<HIDCodes, Vec<(Channel, NoteID)>> {
    let mut mapping = HashMap::new();

    for (chan, mappings) in self.keymapping.iter() {
      for (key, note) in mappings.iter() {
        if let Some(hid_key) = HIDCodes::from_u8(*key) {
          // Try and get the vec if it already is present, if not creat it
          let key_mappings = {
            if let Some(m) = mapping.get_mut(&hid_key) {
              m
            } else {
              mapping.insert(hid_key.clone(), vec![]);
              mapping.get_mut(&hid_key).unwrap()
            }
          };

          key_mappings.push((chan.clone(), note.clone()));
        }
      }
    }

    // self
    //   .keymapping
    //   .iter()
    //   .map(|(key, note)| (HIDCodes::from_u8(*key).unwrap(), *note))
    //   .collect();

    mapping
  }
}

impl Default for AppSettings {
  fn default() -> Self {
    Self {
      keymapping: [(
        0,
        vec![
          (HIDCodes::A as u8, 57),
          (HIDCodes::W as u8, 58),
          (HIDCodes::S as u8, 59),
          (HIDCodes::D as u8, 60),
          (HIDCodes::R as u8, 61),
          (HIDCodes::F as u8, 62),
          (HIDCodes::T as u8, 63),
          (HIDCodes::G as u8, 64),
          (HIDCodes::H as u8, 65),
          (HIDCodes::U as u8, 66),
        ],
      )]
      .iter()
      .cloned()
      .collect(),
      shift_amount: default_shift_amount(),
    }
  }
}
