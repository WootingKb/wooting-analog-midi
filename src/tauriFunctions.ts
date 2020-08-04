import { promisified } from "tauri/api/tauri";
import { HIDCodes } from "./HidCodes";

type PortOption = [number, string, boolean];

export type PortOptions = PortOption[];

export interface AppSettings {
  keymapping: { [channel: string]: [HIDCodes, number][] };
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

export async function getPortOptions(): Promise<PortOptions> {
  return callAppFunction("portOptions");
}

export async function selectPort(option: number): Promise<PortOptions> {
  return callAppFunction("selectPort", { option: option });
}

export async function requestConfig(): Promise<AppSettings> {
  return callAppFunction("requestConfig");
}

export async function updateSettings(settings: AppSettings): Promise<void> {
  //We have to pass it through as a string here because for some reason when it tries to deserialize itself it doesn't like the indexes for the keymap obj
  return callAppFunction("updateConfig", { config: JSON.stringify(settings) });
}
