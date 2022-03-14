import React, { useEffect, useState } from "react";
import { PianoDisplay, MidiDataEntry } from "./PianoDisplay";
import { HIDCodes } from "../HidCodes";
import { MidiUpdate } from "../backend";
import { useSettings } from "../settings-context";
import { useServiceState } from "../state-context";
import { Box, HStack, Select, Text } from "@chakra-ui/react";

interface Props {
  changeMapping: (mapping: [HIDCodes, number][]) => void;
  pianoData: MidiDataEntry[];
  mapping: [HIDCodes, number][];
  midiState: MidiUpdate;
}

let stopBindingTimeoutHandle: number | null = null;

export function Piano(props: Props) {
  // Track if the mouse is pressed so we can avoid playNote triggering with keys
  const [isMousePressed, setIsMousePressed] = useState<number | null>(null);
  const [noteMapping, setNoteMapping] = useState<number | null>(null);

  useEffect(() => {
    if (isMousePressed == null || noteMapping == null) return;

    // Cleanup any existing mappings to this key
    let newMapping = props.mapping.filter(([_, note]) => note !== noteMapping);

    // Left click bind to first pressed key
    if (isMousePressed === 0) {
      const key = Object.keys(props.midiState.data).find(
        (dataKey) => props.midiState.data[dataKey].value > 0.1
      );

      if (!key) return;

      const hidCode = Number(key);

      console.log(`now we can map ${HIDCodes[hidCode]}`);

      // Insert the new mapping
      newMapping.push([hidCode, noteMapping]);
    }

    props.changeMapping(newMapping);
    setNoteMapping(null);
    setIsMousePressed(null);
  }, [noteMapping, props, isMousePressed]);

  return (
    <>
      <Box
        width="90%"
        height="12em"
        padding="1em"
        onContextMenu={(event) => {
          event.preventDefault();
          event.stopPropagation();
        }}
      >
        <PianoDisplay
          midiData={props.pianoData}
          changeMidiMap={(button, midi) => {
            setIsMousePressed(button);
            setNoteMapping(midi);
            if (stopBindingTimeoutHandle !== null) {
              clearTimeout(stopBindingTimeoutHandle);
            }
            stopBindingTimeoutHandle = setTimeout(() => {
              setIsMousePressed(null);
              setNoteMapping(null);
              stopBindingTimeoutHandle = null;
            }, 3000);
          }}
        />
      </Box>
      {noteMapping && isMousePressed === 0 && (
        <div>{`Press a key to bind for MIDI note number ${noteMapping}`}</div>
      )}
    </>
  );
}

export function PianoBody() {
  const [appSettings, appSettingsDispatch] = useSettings();
  const [selectedChannel, setSelectedChannel] = useState<number>(0);
  const serviceState = useServiceState();

  useEffect(() => {
    function cancelKeyEvent(e: KeyboardEvent) {
      // Don't catch Command + Q which is used for quitting
      if (e.code === "KeyQ" && e.metaKey) return;
      e.preventDefault();
    }

    // This may need to be expanded for Arm macs
    if (navigator.platform === "MacIntel") {
      // On mac if we don't catch key events you can hear the system sound
      // https://stackoverflow.com/questions/7992742/how-to-turn-off-keyboard-sounds-in-cocoa-application

      window.addEventListener("keydown", cancelKeyEvent);

      return () => {
        window.removeEventListener("keydown", cancelKeyEvent);
      };
    }
  }, []);

  let pianoData: MidiDataEntry[] = [];

  const channelMapping = appSettings.keymapping[selectedChannel] || [];

  channelMapping.forEach(([key, note_id]) => {
    const entry = serviceState.midiState.data[key];
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
    <>
      <HStack>
        <Text minW="max-content">Current Channel:</Text>
        <Select
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
        </Select>
      </HStack>

      <Piano
        changeMapping={(mapping) =>
          appSettingsDispatch({
            type: "CHANGE_MAPPING",
            mapping,
            channel: selectedChannel,
          })
        }
        pianoData={pianoData}
        mapping={channelMapping}
        midiState={serviceState.midiState}
      />
    </>
  );
}
