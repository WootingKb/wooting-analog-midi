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

let lastData: MidiUpdate = { data: [] };
let updateCallback: Function;
listen<string>("midi-update", function (res) {
  if (updateCallback) {
    const data = JSON.parse(res.payload) as MidiUpdate;
    if (!_.isEqual(data, lastData)) updateCallback(data);
    // Parse lastData again from payload. If we take data after the callacbk the equal check fails for unknown reason
    lastData = JSON.parse(res.payload);
  }
});

function App() {
  const [midiState, setMidiState] = useState<MidiUpdate | undefined>();
  const [appSettings, setAppSettings] = useState<AppSettings | undefined>();

  function updateSettings(settings: AppSettings) {
    setAppSettings(settings);
    tauri.invoke({ cmd: "updateConfig", config: JSON.stringify(settings) });
  }

  useEffect(() => {
    if (!appSettings) {
      tauri
        .promisified<string>({
          cmd: "requestConfig",
        })
        .then(function (response) {
          console.log(response);
          const settings = JSON.parse(response) as AppSettings;
          console.log(settings);
          setAppSettings(settings);

          // settings.keymapping[HIDCodes.ArrowUp] = 20;
          // console.log(settings);
          // updateSettings(settings);
        });
    }
  }, []);

  useEffect(() => {
    updateCallback = (update: MidiUpdate) => {
      setMidiState(update);
    };
  });

  function onClick() {
    //@ts-ignore
    tauri.invoke({
      cmd: "logOperation",
      event: "tauri-click",
      payload: "this payload is optional because we used Option in Rust",
    });

    // if (appSettings) {
    //   appSettings.keymapping[HIDCodes.X] = 41;
    //   console.log(appSettings);
    //   updateSettings(appSettings);
    // }
  }

  const midiData = midiState?.data?.sort((a, b) => a.key - b.key) ?? [];

  const [keyMapping, setKeyMapping] = useState<number | null>(null);

  // Track if the mouse is pressed so we can avoid playNote triggering with keys
  const [isMousePressed, setIsMousePressed] = useState(false);

  useEffect(() => {
    if (keyMapping && isMousePressed) {
      const midiEntry = midiData.find((data) => data.value > 0.1);
      if (midiEntry) {
        console.log(`now we can map ${JSON.stringify(midiEntry)}`);

        setKeyMapping(null);
        setIsMousePressed(false);
      }
    }
  }, [keyMapping, midiData, isMousePressed]);

  return (
    <div className="App">
      <header className="App-header">
        {/* <button onClick={onClick}>Log</button> */}
        <PianoHolder
          onMouseDown={() => {
            setIsMousePressed(true);
            setTimeout(() => {
              setIsMousePressed(false);
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
        {keyMapping && isMousePressed && (
          <div>{`Press a key to bind for MIDI note number ${keyMapping}`}</div>
        )}
      </header>
    </div>
  );
}

export default App;
