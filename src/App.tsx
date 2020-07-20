import "core-js";
import React, { useEffect, useState, useCallback } from "react";
import logo from "./logo.svg";
import * as tauri from "tauri/api/tauri";
import { emit, listen } from "tauri/api/event";

import "./App.css";
import * as _ from "lodash";
import { HIDCodes } from "./HidCodes";
import { PianoDisplay, MidiDataEntry } from "./components/Piano";
import styled from "styled-components";

const PianoHolder = styled.div`
  width: 90%;
  height: 80px;
  padding: 1em;
`;

const Row = styled.div`
  display: flex;
  flex-direction: row;
  align-items: center;
`;

interface AppSettings {
  keymapping: { [channel: number]: [HIDCodes, number][] };
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

interface MidiUpdate {
  data: { [key: number]: MidiUpdateEntry };
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
  const [selectedChannel, setSelectedChannel] = useState<number>(0);

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

  const [noteMapping, setNoteMapping] = useState<number | null>(null);

  // Track if the mouse is pressed so we can avoid playNote triggering with keys
  const [isMousePressed, setIsMousePressed] = useState<number | null>(null);

  useEffect(() => {
    if (appSettings && midiState && noteMapping && isMousePressed != null) {
      // Left click bind to first pressed key
      if (isMousePressed == 0) {
        let key: HIDCodes | undefined;

        for (const x in midiState.data) {
          const entry = midiState.data[x];
          if (entry.value > 0.1) {
            key = parseInt(x) as HIDCodes;
            break;
          }
        }

        if (key) {
          console.log(`now we can map ${HIDCodes[key]}`);

          // Cleanup any existing mappings to this key
          let newMapping = [
            ...(appSettings.keymapping[selectedChannel] ?? []),
          ].filter(([_, note]) => note != noteMapping);

          // Insert the new mapping
          newMapping.push([key, noteMapping]);

          settingsChanged({
            ...appSettings,
            keymapping: {
              ...appSettings.keymapping,
              [selectedChannel]: newMapping,
            },
          });

          setNoteMapping(null);
          setIsMousePressed(null);
        }
      } else if (isMousePressed == 2) {
        //right click unbind
        let newMapping = [
          ...(appSettings.keymapping[selectedChannel] ?? []),
        ].filter(([_, note]) => note != noteMapping);
        settingsChanged({
          ...appSettings,
          keymapping: {
            ...appSettings.keymapping,
            [selectedChannel]: newMapping,
          },
        });
        setNoteMapping(null);
        setIsMousePressed(null);
      }
    }
  }, [noteMapping, midiState, isMousePressed]);

  function onPortSelectionChanged(choice: number) {
    console.log("Selected " + choice);
    selectPort(choice).then((result) => {
      setPortOptions(result);
    });
  }

  let pianoData: MidiDataEntry[] = [];

  if (midiState && appSettings) {
    const channelMapping = appSettings.keymapping[selectedChannel];
    if (channelMapping) {
      channelMapping.forEach(([key, note_id]) => {
        const entry = midiState.data[key];
        // We wanna find a note entry for the currently selected channel and only push it to the Piano if
        const noteEntry = entry.notes?.find(
          (note) => note.channel == selectedChannel && note.note == note_id
        );
        if (noteEntry) {
          pianoData.push({
            key,
            value: entry.value,
            note: noteEntry,
          });
        } else {
          console.error(
            "There should be a Note entry in a midi update for something that's mapped!"
          );
        }
      });
    }
  }

  return (
    <div className="App">
      <header className="App-header">
        <Row>
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
        </Row>

        <Row>
          <p>Current Channel:</p>
          <select
            value={selectedChannel}
            onChange={(event) => {
              setSelectedChannel(parseInt(event.target.value));
            }}
          >
            {[...Array(16).keys()].map((i) => (
              <option key={i} value={i}>
                {i}
              </option>
            ))}
          </select>
        </Row>

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
          {pianoData && (
            <PianoDisplay midiData={pianoData} changeMidiMap={setNoteMapping} />
          )}
        </PianoHolder>
        {noteMapping && isMousePressed == 0 && (
          <div>{`Press a key to bind for MIDI note number ${noteMapping}`}</div>
        )}
      </header>
    </div>
  );
}

export default App;
