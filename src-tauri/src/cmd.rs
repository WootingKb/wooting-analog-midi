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
