import React, { useEffect, useState } from "react";
import logo from "./logo.svg";
import * as tauri from "tauri/api/tauri";
import { emit, listen } from "tauri/api/event";
import "./App.css";

interface MidiEntry {
  key: number;
  note: number;
  value: number;
}

interface MidiUpdate {
  data: MidiEntry[];
}

let updateCallback: Function;
listen<string>("midi-update", function (res) {
  if (updateCallback) updateCallback(JSON.parse(res.payload) as MidiUpdate);
});

function App() {
  const [midiState, setMidiState] = useState<MidiUpdate | undefined>();

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
  }

  return (
    <div className="App">
      <header className="App-header">
        <img src={logo} className="App-logo" alt="logo" />
        <button onClick={onClick}>Log</button>
        {midiState && (
          <div
            style={{
              display: "flex",
              flexDirection: "row",
              width: "100%",
              justifyContent: "space-evenly",
            }}
          >
            {midiState.data
              .sort((a, b) => a.note - b.note)
              .map((value) => {
                return (
                  <div
                    key={value.key}
                    style={{ display: "flex", flexDirection: "column" }}
                  >
                    <p>{value.key}</p>
                    <p>{value.note}</p>
                    <p>{value.value.toPrecision(2)}</p>
                  </div>
                );
              })}
          </div>
        )}
      </header>
    </div>
  );
}

export default App;
