use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
  LogOperation {
    event: String,
    payload: Option<String>,
  },
  PerformRequest {
    endpoint: String,
    body: RequestBody,
    callback: String,
    error: String,
  },

  RequestConfig {
    callback: String,
    error: String,
  },
  UpdateConfig {
    config: String,
  },
}
