import React from "react";
//@ts-ignore
import { Piano } from "react-piano";
import "./piano-styles.css";
import styled from "styled-components";
import { MidiEntry, MIDI_NOTE_MAX, MIDI_NOTE_MIN } from "../backend";
import { HIDCodes } from "../HidCodes";

// import { midiNumberToNote } from "../utils/notes";

export interface MidiDataEntry {
  key: HIDCodes;
  value: number;
  note: MidiEntry;
}

const NoteVelocityMeter = styled.div`
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: flex-end;
  height: 100%;
  background-size: contain;
`;

const NoteKeybind = styled.small<{ isAccidental: boolean }>`
  color: ${(props) => (props.isAccidental ? "#f8e8d5" : "#888")};
  font-size: 12px;
  text-align: center;
  text-transform: capitalize;
  /* Disable text selection */
  user-select: none;
  margin-bottom: 3px;
`;

interface Props {
  midiData: MidiDataEntry[];
  changeMidiMap: (midiNumber: number) => void;
}

// We need to be careful with the rendering of this component. Any rerenders reset animations (like click) in the piano display
export const PianoDisplay = React.memo((props: Props) => {
  const keyboardShortcuts = props.midiData.map((data) => {
    return { key: HIDCodes[data.key], midiNumber: data.note.note };
  });
  const midiLookup = props.midiData.reduce<Map<number, MidiDataEntry>>(
    (acc: Map<number, MidiDataEntry>, entry) => {
      acc.set(entry.note.note, entry);
      return acc;
    },
    new Map()
  );

  return (
    <Piano
      playNote={(midiNumber: number) => props.changeMidiMap(midiNumber)}
      stopNote={() => null}
      noteRange={{ first: MIDI_NOTE_MIN, last: MIDI_NOTE_MAX }}
      renderNoteLabel={(args: {
        keyboardShortcut: string;
        midiNumber: number;
        isActive: boolean;
        isAccidental: boolean;
      }) => {
        const velocity = midiLookup.get(args.midiNumber)?.note?.velocity ?? 0;
        return args.keyboardShortcut ? (
          <NoteVelocityMeter
            style={{
              backgroundImage: `linear-gradient(
            #f28f69 ${velocity * 100}%,
            rgba(0, 0, 0, 0) ${velocity * 100}%
          )`,
            }}
          >
            {/* <NoteKeybind {...args}>
              {midiNumberToNote(args.midiNumber)}
            </NoteKeybind> */}
            <NoteKeybind {...args}>{args.keyboardShortcut}</NoteKeybind>
          </NoteVelocityMeter>
        ) : null;
      }}
      activeNotes={props.midiData
        .filter((data) => {
          return data.note.pressed;
        })
        .map((data) => data.note.note)}
      keyboardShortcuts={keyboardShortcuts}
    />
  );
});
