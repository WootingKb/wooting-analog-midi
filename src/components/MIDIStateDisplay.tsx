import { useColorModeValue } from "@chakra-ui/color-mode";
import _ from "lodash";
import { floor } from "lodash";
import React, { useEffect, useRef, useState } from "react";
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
  width: 3px;
  background-color: lightblue;
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
  maxVelocity: number;
}

const KeyNoteVelocityVisualise = React.memo(
  (props: Props) => {
    const appSettings = useSettingsState();
    const entry = props.entry;
    const key = props.activeKey;
    const value = entry.value;
    const meterBgColor = useColorModeValue("gray", "black");
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
                  ${meterBgColor} ${value * 100}%
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
                <React.Fragment key={id}>
                  <label>{midiNumberToNote(noteEntry.note)}</label>
                  <NoteVelocityMeter
                    style={{
                      backgroundImage: `linear-gradient(
                      to right,
                      rgb(${(1 - velocity) * 255}, ${velocity * 255},0) ${
                        velocity * 100
                      }%,
                      ${meterBgColor} ${velocity * 100}%
                )`,
                    }}
                  >
                    <AnalogThresholdIndicator threshold={props.maxVelocity} />
                  </NoteVelocityMeter>
                </React.Fragment>
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

function usePrevious<T>(value: T): T | undefined {
  const ref = useRef<T>();
  useEffect(() => {
    ref.current = value;
  });
  return ref.current;
}

export function MIDIStateDisplay() {
  const midiState = useMidiState();
  const [activeEntry, setActiveEntry] = useState<
    [string, MidiUpdateEntry] | undefined
  >();
  const previousEntry = usePrevious(activeEntry);
  const [maxVelocity, setMaxVelocity] = useState(0);

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

  useEffect(() => {
    if (activeEntry && activeEntry[1].notes) {
      const entry = activeEntry[1];
      const note = entry.notes[0];
      const previousPressed = previousEntry
        ? (previousEntry[1].notes ?? [])[0]?.pressed ?? false
        : false;

      // If the note has just been triggered on, we want to take the velocity value and use that as a peak (i.e. the velocity at the moment the note was triggered)
      if ((note?.pressed ?? false) && !previousPressed) {
        setMaxVelocity(note.velocity);
      }
    } else {
      setMaxVelocity(0);
    }
  }, [activeEntry, previousEntry]);

  return activeEntry ? (
    <KeyNoteVelocityVisualise
      activeKey={activeEntry[0]}
      entry={activeEntry[1]}
      maxVelocity={maxVelocity}
    />
  ) : (
    <div />
  );
}
