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
use wooting_analog_midi::{FromPrimitive, HIDCodes};

#[derive(Serialize, Deserialize, Debug)]
pub struct AppSettings {
  pub keymapping: HashMap<u8, u8>,
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
