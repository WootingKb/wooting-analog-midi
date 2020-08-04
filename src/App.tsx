import "core-js";
import React, { useEffect, useState } from "react";
import { listen } from "tauri/api/event";

import "./App.css";
import * as _ from "lodash";
import { HIDCodes } from "./HidCodes";
import { MidiDataEntry } from "./components/PianoDisplay";
import styled from "styled-components";
import {
  updateSettings,
  requestConfig,
  getPortOptions,
  selectPort,
  AppSettings,
  PortOptions,
} from "./tauriFunctions";
import { Piano } from "./components/Piano";

const Row = styled.div`
  display: flex;
  flex-direction: row;
  align-items: center;
`;

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
  data: { [key: string]: MidiUpdateEntry };
}

let lastData: MidiUpdate = { data: {} };
let updateCallback: (update: MidiUpdate) => void;
listen<string>("midi-update", function (res) {
  if (updateCallback) {
    const data = JSON.parse(res.payload) as MidiUpdate;
    if (!_.isEqual(data, lastData)) updateCallback(data);
    // Parse lastData again from payload. If we take data after the callacbk the equal check fails for unknown reason
    lastData = JSON.parse(res.payload);
  }
});

function App() {
  const [midiState, setMidiState] = useState<MidiUpdate>({ data: {} });
  const [appSettings, setAppSettings] = useState<AppSettings>({
    keymapping: {},
  });
  const [portOptions, setPortOptions] = useState<PortOptions>([]);
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

    // On mac if we don't catch key events you can hear the system sound
    // https://stackoverflow.com/questions/7992742/how-to-turn-off-keyboard-sounds-in-cocoa-application
    function cancelKeyEvent(e: KeyboardEvent) {
      e.preventDefault();
    }
    window.addEventListener("keydown", cancelKeyEvent);

    return () => {
      window.removeEventListener("keydown", cancelKeyEvent);
    };
  }, []);

  useEffect(() => {
    updateCallback = (update: MidiUpdate) => {
      setMidiState(update);
    };
  });

  const [noteMapping, setNoteMapping] = useState<number | null>(null);

  // Track if the mouse is pressed so we can avoid playNote triggering with keys
  const [isMousePressed, setIsMousePressed] = useState<number | null>(null);

  useEffect(() => {
    if (isMousePressed == null || noteMapping == null) return;

    // Cleanup any existing mappings to this key
    let newMapping = [
      ...(appSettings.keymapping[selectedChannel] ?? []),
    ].filter(([_, note]) => note != noteMapping);

    // Left click bind to first pressed key
    if (isMousePressed == 0) {
      const key = Object.keys(midiState.data).find(
        (dataKey) => midiState.data[dataKey].value > 0.1
      );

      if (!key) return;

      const hidCode = Number(key);

      console.log(`now we can map ${HIDCodes[hidCode]}`);

      // Insert the new mapping
      newMapping.push([hidCode, noteMapping]);
    }

    settingsChanged({
      ...appSettings,
      keymapping: {
        ...appSettings.keymapping,
        [selectedChannel]: newMapping,
      },
    });

    setNoteMapping(null);
    setIsMousePressed(null);
  }, [noteMapping, midiState, isMousePressed]);

  function onPortSelectionChanged(choice: number) {
    console.log("Selected " + choice);
    selectPort(choice).then((result) => {
      setPortOptions(result);
    });
  }

  let pianoData: MidiDataEntry[] = [];

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
          `There should be a Note entry in a midi update for something that's mapped! key:${key} note_id:${note_id}`
        );
      }
    });
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

        <Piano
          setIsMousePressed={setIsMousePressed}
          setNoteMapping={setNoteMapping}
          pianoData={pianoData}
        />

        {noteMapping && isMousePressed == 0 && (
          <div>{`Press a key to bind for MIDI note number ${noteMapping}`}</div>
        )}
      </header>
    </div>
  );
}

export default App;
