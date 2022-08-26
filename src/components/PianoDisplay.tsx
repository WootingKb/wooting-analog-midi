import { Box } from "@chakra-ui/react";
import React from "react";
//@ts-ignore
import { Piano } from "react-piano";
import "react-piano/dist/styles.css";
import { MidiEntry, MIDI_NOTE_MAX, MIDI_NOTE_MIN } from "../backend";
import { HIDCodes } from "../HidCodes";
// import { midiNumberToNote } from "../utils/notes";

export interface MidiDataEntry {
  key: HIDCodes;
  value: number;
  note: MidiEntry;
}

interface Props {
  midiData: MidiDataEntry[];
  changeMidiMap: (mouseButton: number, midiNumber: number) => void;
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
      playNote={() => {}}
      stopNote={() => {}}
      noteRange={{ first: MIDI_NOTE_MIN, last: MIDI_NOTE_MAX }}
      renderNoteLabel={(args: {
        keyboardShortcut: string;
        midiNumber: number;
        isActive: boolean;
        isAccidental: boolean;
      }) => {
        const velocity = midiLookup.get(args.midiNumber)?.note?.velocity ?? 0;
        return (
          <Box
            w="100%"
            h="100%"
            onMouseDown={(event) =>
              props.changeMidiMap(event.button, args.midiNumber)
            }
          >
            {args.keyboardShortcut ? (
              <Box
                display="flex"
                flexDirection="column"
                alignItems="center"
                justifyContent="flex-end"
                height="100%"
                backgroundSize="contain"
                backgroundImage={`linear-gradient(
            #f28f69 ${velocity * 100}%,
            rgba(0, 0, 0, 0) ${velocity * 100}%
          )`}
              >
                <Box
                  color={args.isAccidental ? "#f8e8d5" : "#888"}
                  fontSize="12px"
                  textAlign="center"
                  textTransform="capitalize"
                  /* Disable text selection */
                  userSelect="none"
                  marginBottom="3px"
                >
                  {args.keyboardShortcut}
                  {/* {midiNumberToNote(args.midiNumber)} */}
                </Box>
              </Box>
            ) : null}
          </Box>
        );
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
