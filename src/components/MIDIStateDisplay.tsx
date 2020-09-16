import React from "react";
import styled from "styled-components";
import { MidiUpdate } from "../backend";
import { HIDCodes } from "../HidCodes";
import { midiNumberToNote } from "../utils/notes";

const Grid = styled.div`
  display: grid;
  grid-template-columns: 5em 5em auto;
`;

const AnalogKeyMeter = styled.meter`
  width: 50%;
`;

const NoteVelocityMeter = styled.meter`
  width: 40%;
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

export function MIDIStateDisplay(props: Props) {
  const sorted = Object.entries(props.midiState.data ?? {}).sort(
    (a, b) => b[1].value - a[1].value
  );
  const [key, entry] = sorted[0] ?? [undefined, undefined];
  return (
    <>
      <Grid>
        <p>Key</p>
        <p>Note</p>
        <p>Value</p>
        {key && entry.value > 0.0 && (
          <>
            <KeyLabel index={1} noChildren={entry.notes.length} htmlFor={key}>
              {HIDCodes[parseInt(key)]}
            </KeyLabel>
            <div />
            <AnalogKeyMeter key={key + "m"} id={key} value={entry.value} />
            {entry.notes.map((noteEntry) => {
              const id = `n${noteEntry.note}`;
              return (
                <>
                  <label key={id + "l"} htmlFor={id}>
                    {midiNumberToNote(noteEntry.note)}
                  </label>
                  <NoteVelocityMeter
                    key={id + "m"}
                    id={id}
                    value={noteEntry.velocity}
                  />
                </>
              );
            })}
          </>
        )}
      </Grid>
    </>
  );
}
