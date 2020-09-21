import { floor } from "lodash";
import React, { useEffect, useState } from "react";
import styled from "styled-components";
import { MidiUpdate, MidiUpdateEntry } from "../backend";
import { HIDCodes } from "../HidCodes";
import { useSettingsState } from "../settings-context";
import { midiNumberToNote } from "../utils/notes";

const Grid = styled.div`
  display: grid;
  grid-template-columns: 5em 5em auto;
`;

const AnalogKeyMeter = styled.div`
  width: 50%;
  height: 1em;
`;

const NoteVelocityMeter = styled.div`
  width: 40%;
  height: 0.5em;
  align-self: center;
`;

const AnalogThresholdIndicator = styled.div<{ threshold: number }>`
  width: 2px;
  background-color: black;
  left: ${(props) => floor(props.threshold * 100)}%;
  height: 100%;
  position: relative;
`;

interface KeyEntryProps {
  index: number;
  noChildren: number;
}

const KeyLabel = styled.label<KeyEntryProps>`
  grid-row: ${(props) => props.index + 1} / span
    ${(props) => props.noChildren + 1};
`;

interface Props {
  midiState: MidiUpdate;
}

// export function MIDIStateDisplay(props: Props) {
//   let rows = 1;
//   return (
//     <>
//       <Grid>
//         <p>Key</p>
//         <p>Note</p>
//         <p>Value</p>
//         {Object.entries(props.midiState.data).map(([key, entry], i) => {
//           const workingRow = rows;
//           rows += entry.notes.length + 1;
//           return entry.value >= 0.0 ? (
//             <>
//               <KeyLabel
//                 index={workingRow}
//                 noChildren={entry.notes.length}
//                 htmlFor={key}
//               >
//                 {HIDCodes[parseInt(key)]}
//               </KeyLabel>
//               <div />
//               <AnalogKeyMeter id={key} value={entry.value} />
//               {entry.notes.map((noteEntry) => {
//                 const id = `n${noteEntry.note}`;
//                 return (
//                   <>
//                     <label htmlFor={id}>{noteEntry.note}</label>
//                     <NoteVelocityMeter id={id} value={noteEntry.velocity} />
//                   </>
//                 );
//               })}
//             </>
//           ) : (
//             <></>
//           );
//         })}
//       </Grid>
//     </>
//   );
// }

export const MIDIStateDisplay = React.memo((props: Props) => {
  const appSettings = useSettingsState();
  const [activeEntry, setActiveEntry] = useState<
    [string, MidiUpdateEntry] | undefined
  >();

  useEffect(() => {
    const sorted = Object.entries(props.midiState.data ?? {}).sort(
      (a, b) => b[1].value - a[1].value
    );
    const mostPressed = sorted[0];
    if (mostPressed && (!activeEntry || mostPressed[1].value > 0.0)) {
      setActiveEntry(mostPressed);
    } else if (activeEntry && (!mostPressed || mostPressed[1].value === 0.0)) {
      const emptyEntry = props.midiState.data[activeEntry[0]] ?? {
        ...activeEntry,
        value: 0.0,
      };
      setActiveEntry((a) => [activeEntry[0], emptyEntry]);
    }
    // eslint-disable-next-line
  }, [props.midiState]);

  const [key, entry] = activeEntry ?? [undefined, undefined];
  const value = entry?.value ?? 0;
  return (
    <>
      <Grid>
        <p>Key</p>
        <p>Note</p>
        <p>Value</p>
        {key && entry && (
          <>
            <KeyLabel
              index={1}
              noChildren={entry.notes?.length ?? 0}
              htmlFor={key}
            >
              {HIDCodes[parseInt(key)]}
            </KeyLabel>
            <div />
            <AnalogKeyMeter
              key={key + "m"}
              style={{
                backgroundImage: `linear-gradient(
                        to right,
                    ${
                      value < appSettings.note_config.threshold
                        ? "red"
                        : "rgb(0, 255, 0)"
                      // : `rgb(${(1 - value) * 255}, ${value * 255},0)`
                    } ${value * 100}%,
                    white ${value * 100}%
                  )`,
              }}
            >
              <AnalogThresholdIndicator
                threshold={appSettings.note_config.threshold}
              />
            </AnalogKeyMeter>
            {(entry.notes ?? []).map((noteEntry) => {
              const id = `n${noteEntry.note}`;
              const velocity = noteEntry.velocity;
              return (
                <>
                  <label key={id + "l"}>
                    {midiNumberToNote(noteEntry.note)}
                  </label>
                  <NoteVelocityMeter
                    key={id + "m"}
                    style={{
                      backgroundImage: `linear-gradient(
                        to right,
                        rgb(${(1 - velocity) * 255}, ${velocity * 255},0) ${
                        velocity * 100
                      }%,
                        white ${velocity * 100}%
                  )`,
                    }}
                  />
                </>
              );
            })}
          </>
        )}
      </Grid>
    </>
  );
});
