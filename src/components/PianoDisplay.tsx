import React from "react";
//@ts-ignore
import { Piano } from "react-piano";
import "react-piano/dist/styles.css";
import { MidiEntry } from "../backend";
import { HIDCodes } from "../HidCodes";

export interface MidiDataEntry {
  key: HIDCodes;
  value: number;
  note: MidiEntry;
}

interface Props {
  midiData: MidiDataEntry[];
  changeMidiMap: (midiNumber: number) => void;
}

// We need to be careful with the rendering of this component. Any rerenders reset animations (like click) in the piano display
export const PianoDisplay = React.memo((props: Props) => {
  const keyboardShortcuts = props.midiData.map((data) => {
    return { key: HIDCodes[data.key], midiNumber: data.note.note };
  });

  return (
    <Piano
      playNote={(midiNumber: number) => props.changeMidiMap(midiNumber)}
      stopNote={() => null}
      noteRange={{ first: 21, last: 108 }}
      activeNotes={props.midiData
        .filter((data) => {
          return data.note.pressed;
        })
        .map((data) => data.note.note)}
      keyboardShortcuts={keyboardShortcuts}
    />
  );
});
