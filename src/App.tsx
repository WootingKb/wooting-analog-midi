import "core-js";
import React, { useEffect, useState } from "react";
import { MidiDataEntry } from "./components/PianoDisplay";
import styled from "styled-components";
import {
  AppSettings,
  PortOptions,
  backend,
  MidiUpdate,
  DeviceList,
  MIDI_NOTE_MAX,
} from "./backend";
import { Piano } from "./components/Piano";

const Row = styled.div`
  display: flex;
  flex-direction: row;
  align-items: center;
`;

const Column = styled.div`
  display: flex;
  flex-direction: column;
  align-items: center;
`;

const AppRoot = styled.div`
  padding: 1em;
`;

const Heading = styled.div`
  display: flex;
  flex-direction: row;
  justify-content: space-between;
`;

const Body = styled.div`
  display: flex;
  flex-direction: column;
  align-items: center;
`;

const SettingsBody = styled.div`
  display: flex;
  flex-direction: row;
  flex-wrap: wrap;
`;

const NumberInput = styled.input`
  text-align: center;
`;

function App() {
  const [midiState, setMidiState] = useState<MidiUpdate>({ data: {} });
  const [appSettings, setAppSettings] = useState<AppSettings>({
    keymapping: {},
    shift_amount: 0,
  });
  const [portOptions, setPortOptions] = useState<PortOptions>([]);
  const [selectedChannel, setSelectedChannel] = useState<number>(0);
  const [connectedDevices, setConnectedDevices] = useState<DeviceList>(
    backend.connectedDeviceList
  );

  function settingsChanged(settings: AppSettings) {
    setAppSettings(settings);
    backend.updateSettings(settings);
  }

  useEffect(() => {
    backend.on("found-devices", (devices: DeviceList) => {
      setConnectedDevices(devices);
    });

    backend.on("no-devices", () => {
      setConnectedDevices([]);
    });

    return () => {
      // TODO: Cleanup this so it's not removing all listeners
      backend.removeAllListeners("found-devices");
      backend.removeAllListeners("no-devices");
    };
  });

  useEffect(() => {
    backend.onInitComplete(() => {
      console.log("Init complete");
      backend.requestConfig().then(function (settings) {
        setAppSettings(settings);

        // settings.keymapping[HIDCodes.ArrowUp] = 20;
        // console.log(settings);
        // updateSettings(settings);
      });

      backend.getPortOptions().then((result) => {
        console.log(result);
        setPortOptions(result);
      });
    });

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
    backend.on("midi-update", (update: MidiUpdate) => {
      setMidiState(update);
    });

    return () => {
      //TODO: Cleanup this unsubscribe
      backend.removeAllListeners("midi-update");
    };
  }, []);

  function onPortSelectionChanged(choice: number) {
    console.log("Selected " + choice);
    backend.selectPort(choice).then((result) => {
      setPortOptions(result);
    });
  }

  let pianoData: MidiDataEntry[] = [];

  const channelMapping = appSettings.keymapping[selectedChannel] || [];

  channelMapping.forEach(([key, note_id]) => {
    const entry = midiState.data[key];
    // We wanna find a note entry for the currently selected channel and only push it to the Piano if
    if (!entry) return;

    const noteEntry = entry.notes?.find(
      (note) => note.channel === selectedChannel && note.note === note_id
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

  return (
    <AppRoot>
      <Heading>
        <p>
          {connectedDevices.length > 0
            ? `Device '${connectedDevices[0].device_name}' is connected, you're all set!`
            : "No compatible devices could be found!"}
        </p>
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
      </Heading>
      <Body>
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
          changeMapping={(mapping) =>
            settingsChanged({
              ...appSettings,
              keymapping: {
                ...appSettings.keymapping,
                [selectedChannel]: mapping,
              },
            })
          }
          pianoData={pianoData}
          mapping={channelMapping}
          midiState={midiState}
        />
      </Body>
      <SettingsBody>
        <Column>
          <p>Shift Amount</p>

          <NumberInput
            type="number"
            value={appSettings.shift_amount}
            onChange={(event) => {
              settingsChanged({
                ...appSettings,
                shift_amount: parseInt(event.target.value),
              });
            }}
            min={-MIDI_NOTE_MAX}
            max={MIDI_NOTE_MAX}
          />
        </Column>
      </SettingsBody>
    </AppRoot>
  );
}

export default App;
