import React, { useEffect, useState, useCallback } from "react";
import logo from "./logo.svg";
import * as tauri from "tauri/api/tauri";
import { emit, listen } from "tauri/api/event";

import "./App.css";
import * as _ from "lodash";
import { HIDCodes } from "./HidCodes";
import { PianoDisplay } from "./components/Piano";
import styled from "styled-components";

const PianoHolder = styled.div`
  width: 90%;
  height: 80px;
  padding: 1em;
`;

const PortSelectionWrapper = styled.div`
  display: flex;
  flex-direction: row;
  align-items: center;
`;

interface AppSettings {
  keymapping: { [key: number]: number };
}

export interface MidiEntry {
  key: HIDCodes;
  note?: number;
  value: number;
}

interface MidiUpdate {
  data: MidiEntry[];
}

type PortOption = [number, string, boolean];

type PortOptions = PortOption[];

async function callAppFunction<T>(name: string, args?: any): Promise<T> {
  return await tauri.promisified<T>({
    cmd: "function",
    call: {
      func: name,
      ...args,
    },
  });
}

async function getPortOptions(): Promise<PortOptions> {
  return callAppFunction("portOptions");
}

async function selectPort(option: number): Promise<PortOptions> {
  return callAppFunction("selectPort", { option: option });
}

async function requestConfig(): Promise<AppSettings> {
  return callAppFunction("requestConfig");
}

async function updateSettings(settings: AppSettings): Promise<void> {
  //We have to pass it through as a string here because for some reason when it tries to deserialize itself it doesn't like the indexes for the keymap obj
  return callAppFunction("updateConfig", { config: JSON.stringify(settings) });
}

let lastData: MidiUpdate = { data: [] };
let updateCallback: (update: MidiUpdate) => void;
listen<string>("midi-update", function (res) {
  if (updateCallback) {
    const data = JSON.parse(res.payload) as MidiUpdate;
    if (!_.isEqual(data, lastData)) updateCallback(data);
    delete lastData.data;
    // Parse lastData again from payload. If we take data after the callacbk the equal check fails for unknown reason
    lastData = JSON.parse(res.payload);
    delete res.payload;
  }
});

function App() {
  const [midiState, setMidiState] = useState<MidiUpdate | undefined>();
  const [appSettings, setAppSettings] = useState<AppSettings | undefined>();
  const [portOptions, setPortOptions] = useState<PortOptions | undefined>();

  function settingsChanged(settings: AppSettings) {
    setAppSettings(settings);
    updateSettings(settings);
  }

  useEffect(() => {
    listen(
      "init-complete",
      () => {
        console.log("Init complete");
        requestConfig().then(function (settings) {
          setAppSettings(settings);

          // settings.keymapping[HIDCodes.ArrowUp] = 20;
          // console.log(settings);
          // updateSettings(settings);
        });
        getPortOptions().then((result) => {
          console.log(result);
          setPortOptions(result);
        });
      },
      true
    );
  }, []);

  useEffect(() => {
    updateCallback = (update: MidiUpdate) => {
      if (midiState) {
        delete midiState.data;
      }
      setMidiState(update);
    };
  });

  const midiData = midiState?.data?.sort((a, b) => a.key - b.key) ?? [];

  const [keyMapping, setKeyMapping] = useState<number | null>(null);

  // Track if the mouse is pressed so we can avoid playNote triggering with keys
  const [isMousePressed, setIsMousePressed] = useState<number | null>(null);

  useEffect(() => {
    if (appSettings && keyMapping && isMousePressed != null) {
      // Left click bind to first pressed key
      if (isMousePressed == 0) {
        const midiEntry = midiData.find((data) => data.value > 0.1);
        if (midiEntry) {
          console.log(`now we can map ${JSON.stringify(midiEntry)}`);

          settingsChanged({
            ...appSettings,
            keymapping: {
              ...appSettings.keymapping,
              [midiEntry.key]: keyMapping,
            },
          });

          setKeyMapping(null);
          setIsMousePressed(null);
        }
      } else if (isMousePressed == 2) {
        //right click unbind
        let newMapping = { ...appSettings.keymapping };
        for (const x in newMapping) {
          if (newMapping[x] == keyMapping) {
            delete newMapping[x];
            break;
          }
        }
        settingsChanged({
          ...appSettings,
          keymapping: newMapping,
        });
        setKeyMapping(null);
        setIsMousePressed(null);
      }
    }
  }, [keyMapping, midiData, isMousePressed]);

  function onPortSelectionChanged(choice: number) {
    console.log("Selected " + choice);
    selectPort(choice).then((result) => {
      setPortOptions(result);
    });
  }

  return (
    <div className="App">
      <header className="App-header">
        <PortSelectionWrapper>
          <p>Output Port:</p>
          {portOptions && (
            <select
              value={portOptions.findIndex((item) => item[2])}
              onChange={(event) => {
                onPortSelectionChanged(parseInt(event.target.value));
              }}
            >
              {portOptions.map((item) => (
                <option key={item[0]} value={item[0]}>
                  {item[1]}
                </option>
              ))}
            </select>
          )}
        </PortSelectionWrapper>

        {/* <button onClick={onClick}>Log</button> */}
        <PianoHolder
          onMouseDown={(event) => {
            // event.stopPropagation();
            setIsMousePressed(event.button);
            setTimeout(() => {
              setIsMousePressed(null);
            }, 3000);
          }}
        >
          {midiState && (
            <PianoDisplay
              midiData={midiData.filter((data) => data.note)}
              changeMidiMap={setKeyMapping}
            />
          )}
        </PianoHolder>
        {keyMapping && isMousePressed == 0 && (
          <div>{`Press a key to bind for MIDI note number ${keyMapping}`}</div>
        )}
      </header>
    </div>
  );
}

export default App;
