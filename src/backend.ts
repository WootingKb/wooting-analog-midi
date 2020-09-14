import { promisified } from "tauri/api/tauri";
import { HIDCodes } from "./HidCodes";
import { EventEmitter } from "events";
import { listen } from "tauri/api/event";
import * as _ from "lodash";

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

export interface AppSettings {
  keymapping: { [channel: string]: [HIDCodes, number][] };
  shift_amount: number;
}

export interface MidiEntry {
  note: number;
  value: number;
  channel: number;
  pressed: boolean;
}

interface MidiUpdateEntry {
  value: number;
  notes: MidiEntry[];
}

export interface MidiUpdate {
  data: { [key: string]: MidiUpdateEntry };
}

async function callAppFunction<T>(name: string, args?: any): Promise<T> {
  return await promisified<T>({
    cmd: "function",
    call: {
      func: name,
      ...args,
    },
  });
}

export class Backend extends EventEmitter {
  static instance: Backend = new Backend();
  private lastMidi: MidiUpdate;
  public hasDevices: boolean;
  public hasInitComplete: boolean;
  public connectedDeviceList: DeviceList;

  constructor() {
    super();

    // Handle listening for midi-update
    this.lastMidi = { data: {} };
    listen<string>("midi-update", (res) => {
      const data = JSON.parse(res.payload) as MidiUpdate;
      if (!_.isEqual(data, this.lastMidi)) this.emit("midi-update", data);
      // Parse lastData again from payload. If we take data after the callacbk the equal check fails for unknown reason
      this.lastMidi = JSON.parse(res.payload);
    });
    this.hasDevices = false;
    this.connectedDeviceList = [];
    listen<string>("found-devices", (res) => {
      console.log("Found devices");
      this.connectedDeviceList = JSON.parse(res.payload) as DeviceList;
      this.hasDevices = true;
      this.emit("found-devices", this.connectedDeviceList);
    });

    listen<string>("no-devices", (res) => {
      console.log("No devices");
      this.hasDevices = false;
      this.connectedDeviceList = [];
      this.emit("no-devices");
    });

    this.hasInitComplete = false;
    listen<string>("init-complete", (res) => {
      console.log("Received init complete");
      this.hasInitComplete = true;
      this.emit("init-complete");
    });
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
