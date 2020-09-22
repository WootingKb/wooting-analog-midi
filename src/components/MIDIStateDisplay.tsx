import _ from "lodash";
import { floor } from "lodash";
import React, { useEffect, useState } from "react";
import styled from "styled-components";
import { MidiUpdateEntry } from "../backend";
import { HIDCodes } from "../HidCodes";
import { useSettingsState } from "../settings-context";
import { useMidiState } from "../state-context";
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
  activeKey: string;
  entry: MidiUpdateEntry;
}

const KeyNoteVelocityVisualise = React.memo(
  (props: Props) => {
    const appSettings = useSettingsState();
    const entry = props.entry;
    const key = props.activeKey;
    const value = entry.value;

    return (
      <>
        <Grid>
          <p>Key</p>
          <p>Note</p>
          <p>Value</p>
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
        </Grid>
      </>
    );
  },
  (a, b) => {
    // console.log("Checking equality between ", a, b);
    return _.isEqual(a, b);
  }
);

export function MIDIStateDisplay() {
  const midiState = useMidiState();
  const [activeEntry, setActiveEntry] = useState<
    [string, MidiUpdateEntry] | undefined
  >();

  useEffect(() => {
    const sorted = Object.entries(midiState.data ?? {}).sort(
      (a, b) => b[1].value - a[1].value
    );
    const mostPressed = sorted[0];
    if (mostPressed && (!activeEntry || mostPressed[1].value > 0.0)) {
      setActiveEntry(mostPressed);
    } else if (activeEntry && (!mostPressed || mostPressed[1].value === 0.0)) {
      // Only update the current one to an empty entry if it's not already empty
      if (activeEntry[1].value > 0.0) {
        const emptyEntry = midiState.data[activeEntry[0]] ?? {
          ...activeEntry,
          value: 0.0,
        };
        setActiveEntry([activeEntry[0], emptyEntry]);
      }
    }
    // eslint-disable-next-line
  }, [midiState]);

  return activeEntry ? (
    <KeyNoteVelocityVisualise
      activeKey={activeEntry[0]}
      entry={activeEntry[1]}
    />
  ) : (
    <div />
  );
}
