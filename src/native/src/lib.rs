mod app;
mod settings;

extern crate env_logger;
extern crate wooting_analog_midi;
#[macro_use]
extern crate lazy_static;
// #[macro_use]
// extern crate crossbeam_channel;
#[allow(unused_imports)]
#[macro_use]
extern crate anyhow;

use app::{App, AppFunction};
use log::{error, info, warn};
use neon::prelude::*;
use serde_json;
use std::sync::{Arc, RwLock};

lazy_static! {
  static ref APP: RwLock<App> = RwLock::new(App::new());
}

fn hello(mut cx: FunctionContext) -> JsResult<JsString> {
  Ok(cx.string("hello node"))
}

fn app_command(mut cx: FunctionContext) -> JsResult<JsUndefined> {
  let arg = cx.argument::<JsString>(0)?.value(&mut cx);
  let callback = cx.argument::<JsFunction>(1)?.root(&mut cx);
  let queue = cx.queue();
  std::thread::spawn(move || {
    use anyhow::Context;
    let result = match serde_json::from_str::<AppFunction>(&arg[..]) {
      Err(e) => Err(anyhow!(e.to_string())),
      Ok(command) => APP
        .write()
        .unwrap()
        .process_command(command)
        .and_then(|value| {
          serde_json::to_string(&value)
            .context("Failed to serialize command result value as string")
        }),
    };

    queue.send(move |mut cx| {
      let callback = callback.into_inner(&mut cx);
      let this = cx.undefined();
      let args = match result {
        Ok(response) => vec![cx.null().upcast::<JsValue>(), cx.string(response).upcast()],
        Err(e) => vec![
          cx.string(format!("{:#?}", e)).upcast::<JsValue>(),
          cx.null().upcast(),
        ],
      };

      callback.call(&mut cx, this, args)?;

      Ok(())
    });
  });

  Ok(cx.undefined())
}

fn init_app(mut cx: FunctionContext) -> JsResult<JsUndefined> {
  if let Err(e) = env_logger::try_init() {
    warn!("Failed to init env_logger, {:#?}", e);
  }

  let callback = cx.argument::<JsFunction>(0)?.root(&mut cx);
  let queue = cx.queue();

  let event_receiver = match APP.write().unwrap().init() {
    Ok(receiver) => receiver,
    Err(e) => {
      error!("{:#?}", e);
      panic!("App Init failed {:#?}", e);
    }
  };
  let callback = Arc::new(RwLock::new(callback));
  APP.write().unwrap().exec_loop(move || {
    if let Ok(event) = event_receiver.recv()
    // .map_err(|err| warn!("Error on event reciever: {}", err))
    {
      let callback_inner = callback.clone();
      queue.send(move |mut cx| {
        let callback = callback_inner.write().unwrap().to_inner(&mut cx);
        let this = cx.undefined();
        let args = vec![cx
          .string(serde_json::to_string(&event).expect("Failed to serialize event"))
          .upcast::<JsValue>()];

        callback.call(&mut cx, this, args)?;

        Ok(())
      });
    }
  });

  Ok(cx.undefined())
}

register_module!(mut cx, {
  cx.export_function("hello", hello)?;
  cx.export_function("app_command", app_command)?;
  cx.export_function("init_app", init_app)?;
  Ok(())
});
