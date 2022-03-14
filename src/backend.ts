import { invoke } from "@tauri-apps/api/tauri";
import { HIDCodes } from "./HidCodes";
import { EventEmitter } from "events";
import { listen } from "@tauri-apps/api/event";

import { SettingsDispatch } from "./settings-context";
import { ServiceStateAction, ServiceStateDispatch } from "./state-context";

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
  return await invoke<T>(name, {
    ...args,
  });
}

export class Backend extends EventEmitter {
  static instance: Backend = new Backend();

  public settingsDispatcher?: SettingsDispatch;
  public serviceDispatcher?: ServiceStateDispatch;
  serviceActionsQueue: ServiceStateAction[] = [];

  constructor() {
    super();

    listen<string>("event", (res) => {
      const payload = JSON.parse(res.payload) as ServiceStateAction;
      // console.log("Received event ", payload);
      this.dispatchEvent(payload);
    });
    
    this.getPortOptions().then((value) => {
      this.serviceDispatcher!({ type: "PORT_OPTIONS", value });
    });

    this.getConnectedDevices().then((value) => {
      this.serviceDispatcher!({ type: "FOUND_DEVICES", value });
    });
  }

  dispatchEvent = (action: ServiceStateAction) => {
    if (this.serviceDispatcher) {
      this.serviceDispatcher(action);
    } else {
      console.log("Putting it in queue because we don't have a dispatcher");
      this.serviceActionsQueue.push(action);
    }
  };

  setSettingsDispatcher(dispatch: SettingsDispatch) {
    if (!this.settingsDispatcher) {
      this.settingsDispatcher = dispatch;

      this.requestConfig().then((settings) => {
        console.log("requested ", settings);
        dispatch({ type: "INIT", value: settings });
      });
    } else {
      this.settingsDispatcher = dispatch;
    }
  }

  setServiceDispatcher(dispatch: ServiceStateDispatch) {
    this.serviceDispatcher = dispatch;
    if (this.serviceActionsQueue.length > 0) {
      this.serviceActionsQueue.forEach((element) => {
        dispatch(element);
      });
      this.serviceActionsQueue = [];
    }
  }

  async getPortOptions(): Promise<PortOptions> {
    return callAppFunction("get_port_options");
  }

  async getConnectedDevices(): Promise<DeviceInfo[]> {
    return callAppFunction("get_connected_devices");
  }

  async selectPort(option: number): Promise<PortOptions> {
    return callAppFunction<PortOptions>("select_port", {
      option: option,
    });
  }

  async requestConfig(): Promise<AppSettings> {
    return callAppFunction("get_config");
  }

  async updateSettings(settings: AppSettings): Promise<void> {
    //We have to pass it through as a string here because for some reason when it tries to deserialize itself it doesn't like the indexes for the keymap obj
    return callAppFunction("update_config", {
      config: settings,
    });
  }
}

export const backend = Backend.instance;
