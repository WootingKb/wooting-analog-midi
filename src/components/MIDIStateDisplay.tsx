import { useColorModeValue } from "@chakra-ui/color-mode";
import { Box, chakra, Grid } from "@chakra-ui/react";
import _ from "lodash";
import { floor } from "lodash";
import React, { useEffect, useRef, useState } from "react";
import { MidiUpdateEntry } from "../backend";
import { HIDCodes } from "../HidCodes";
import { useSettingsState } from "../settings-context";
import { useMidiState } from "../state-context";
import { midiNumberToNote } from "../utils/notes";

interface Props {
  activeKey: string;
  entry: MidiUpdateEntry;
  maxVelocity: number;
}

function AnalogThresholdIndicator(props: { threshold: number }) {
  return (
    <Box
      width="3px"
      backgroundColor="lightblue"
      left={`${floor(props.threshold * 100)}%`}
      height="100%"
      position="relative"
    />
  );
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
        <Grid gridTemplateColumns="5em 5em auto">
          <p>Key</p>
          <p>Note</p>
          <p>Value</p>
          <>
            <chakra.label
              index={1}
              noChildren={entry.notes?.length ?? 0}
              gridRow={`2 / span ${(entry.notes?.length ?? 0) + 1}`}
              htmlFor={key}
            >
              {HIDCodes[parseInt(key)]}
            </chakra.label>
            <div />
            <Box
              key={key + "m"}
              width="50%"
              height="1em"
              backgroundImage={`linear-gradient(
                      to right,
                  ${
                    value < appSettings.note_config.threshold
                      ? "red"
                      : "rgb(0, 255, 0)"
                    // : `rgb(${(1 - value) * 255}, ${value * 255},0)`
                  } ${value * 100}%,
                  ${meterBgColor} ${value * 100}%
                )`}
            >
              <AnalogThresholdIndicator
                threshold={appSettings.note_config.threshold}
              />
            </Box>
            {(entry.notes ?? []).map((noteEntry) => {
              const id = `n${noteEntry.note}`;
              const velocity = noteEntry.velocity;
              return (
                <React.Fragment key={id}>
                  <label>{midiNumberToNote(noteEntry.note)}</label>
                  <Box
                    w="40%"
                    h="0.5em"
                    alignSelf="center"
                    backgroundImage={`linear-gradient(
                    to right,
                    rgb(${(1 - velocity) * 255}, ${velocity * 255},0) ${
                      velocity * 100
                    }%,
                    ${meterBgColor} ${velocity * 100}%
              )`}
                  >
                    <AnalogThresholdIndicator threshold={props.maxVelocity} />
                  </Box>
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
