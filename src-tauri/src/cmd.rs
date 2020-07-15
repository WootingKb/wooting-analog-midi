use serde::{Deserialize, Serialize};
use std::collections::HashMap;
// use wooting_analog_midi::PortOption;

#[derive(Serialize, Deserialize, Debug)]
pub struct AppSettings {
  pub keymapping: HashMap<u8, u8>,
}

#[derive(Debug, Deserialize)]
pub struct RequestBody {
  id: i32,
  name: String,
}

#[derive(Deserialize)]
#[serde(tag = "cmd", rename_all = "camelCase")]
pub enum Cmd {
  RequestConfig {
    callback: String,
    error: String,
  },
  UpdateConfig {
    config: String,
  },
  PortOptions {
    callback: String,
    error: String,
  },
  SelectPort {
    option: usize,
    callback: String,
    error: String,
  },
}
