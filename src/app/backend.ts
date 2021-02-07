// import { promisified } from "tauri/api/tauri";
import { HIDCodes } from "./HidCodes";
import { EventEmitter } from "events";
// import { listen } from "tauri/api/event";

import { SettingsDispatch } from "./settings-context";
import { ServiceStateAction, ServiceStateDispatch } from "./state-context";
import { app_command, app_command_promise, hello, init_app } from "../native";

type PortOption = [number, string, boolean];

export const MIDI_NOTE_MIN = 21;
export const MIDI_NOTE_MAX = 108;

export type PortOptions = PortOption[];

export enum DeviceType {
  /// Device is of type Keyboard
  Keyboard = 1,
  /// Device is of type Keypad
  Keypad = 2,
  /// Device
  Other = 3,
}

export interface DeviceInfo {
  /// Device Vendor ID `vid`
  vendor_id: number;
  /// Device Product ID `pid`
  product_id: number;
  /// Device Manufacturer name
  manufacturer_name: String;
  /// Device name
  device_name: String;
  /// Unique device ID
  device_id: number;
  /// Hardware type of the Device
  device_type: DeviceType;
}

export type DeviceList = DeviceInfo[];

export interface NoteConfig {
  threshold: number;
  velocity_scale: number;
}

export interface AppSettings {
  keymapping: { [channel: string]: [HIDCodes, number][] };
  shift_amount: number;
  note_config: NoteConfig;
}

export interface MidiEntry {
  note: number;
  velocity: number;
  channel: number;
  pressed: boolean;
}

export interface MidiUpdateEntry {
  value: number;
  notes: MidiEntry[];
}

export interface MidiUpdate {
  data: { [key: string]: MidiUpdateEntry };
}

async function callAppFunction<T>(name: string, args?: any): Promise<T> {
  return await app_command_promise(
    JSON.stringify({ func: name, ...args })
  ).then((result) => JSON.parse(result));
}

export class Backend extends EventEmitter {
  static instance: Backend = new Backend();
  public hasInitComplete: boolean;
  public settingsDispatcher?: SettingsDispatch;
  public serviceDispatcher?: ServiceStateDispatch;
  serviceActionsQueue: ServiceStateAction[] = [];

  constructor() {
    super();
    console.log(hello());

    init_app((event) => {
      const payload = JSON.parse(event) as ServiceStateAction;
      // console.log("Received event ", payload);
      if (this.serviceDispatcher) {
        this.serviceDispatcher(payload);
      } else {
        console.log("Putting it in queue because we don't have a dispatcher");
        this.serviceActionsQueue.push(payload);
      }
    });
    this.emit("init-complete");
    this.hasInitComplete = true;
  }

  setSettingsDispatcher(dispatch: SettingsDispatch) {
    if (!this.settingsDispatcher) {
      this.settingsDispatcher = dispatch;
      this.onInitComplete(() => {
        this.requestConfig().then((settings) => {
          // console.log("requested ", settings);
          dispatch({ type: "INIT", value: settings });
        });
      });
    } else {
      this.settingsDispatcher = dispatch;
    }
  }

  setServiceDispatcher(dispatch: ServiceStateDispatch) {
    // if (!this.serviceDispatcher) {
    //   this.settingsDispatcher = dispatch;
    // } else {
    this.serviceDispatcher = dispatch;
    if (this.serviceActionsQueue.length > 0) {
      this.serviceActionsQueue.forEach((element) => {
        dispatch(element);
      });
      this.serviceActionsQueue = [];
    }
    // }
  }

  async getPortOptions(): Promise<PortOptions> {
    return callAppFunction("portOptions");
  }

  async selectPort(option: number): Promise<PortOptions> {
    return callAppFunction("selectPort", { option: option });
  }

  async requestConfig(): Promise<AppSettings> {
    return callAppFunction("requestConfig");
  }

  async updateSettings(settings: AppSettings): Promise<void> {
    //We have to pass it through as a string here because for some reason when it tries to deserialize itself it doesn't like the indexes for the keymap obj
    return callAppFunction("updateConfig", {
      config: JSON.stringify(settings),
    });
  }

  onInitComplete(cb: () => void) {
    // This is to ensure that the callback gets retroactively called if it gets added after the init complete event has already happened
    if (this.hasInitComplete) {
      cb();
    } else {
      this.once("init-complete", cb);
    }
  }
}

export const backend = Backend.instance;
