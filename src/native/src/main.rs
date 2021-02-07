extern crate ctrlc;
mod app;
mod settings;
use crate::app::App;
use anyhow::Result;
use log::warn;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn main() -> Result<()> {
  if let Err(e) = env_logger::try_init() {
    warn!("Failed to init env_logger, {:#?}", e);
  }
  let running = Arc::new(AtomicBool::new(true));
  let r = running.clone();

  ctrlc::set_handler(move || {
    r.store(false, Ordering::SeqCst);
  })
  .expect("Error setting Ctrl-C handler");
  let mut app = App::new();
  let receiver = app.init()?;
  app.exec_loop(move || {
    if let Ok(event) = receiver.recv()
    // .map_err(|err| warn!("Error on event reciever: {}", err))
    {}
  });

  println!("Waiting for Ctrl-C...");
  while running.load(Ordering::SeqCst) {}
  println!("Got it! Exiting...");
  drop(app);

  Ok(())
}
