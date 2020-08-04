import React, { useEffect, useState } from "react";
import styled from "styled-components";
import { PianoDisplay, MidiDataEntry } from "./PianoDisplay";
import { HIDCodes } from "../HidCodes";
import { MidiUpdate } from "../App";

const PianoHolder = styled.div`
  width: 90%;
  height: 80px;
  padding: 1em;
`;

interface Props {
  changeMapping: (mapping: [HIDCodes, number][]) => void;
  pianoData: MidiDataEntry[];
  mapping: [HIDCodes, number][];
  midiState: MidiUpdate;
}

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
      <PianoHolder
        onMouseDown={(event) => {
          // event.stopPropagation();
          setIsMousePressed(event.button);
          setTimeout(() => {
            setIsMousePressed(null);
          }, 3000);
        }}
      >
        <PianoDisplay
          midiData={props.pianoData}
          changeMidiMap={setNoteMapping}
        />
      </PianoHolder>
      {noteMapping && isMousePressed === 0 && (
        <div>{`Press a key to bind for MIDI note number ${noteMapping}`}</div>
      )}
    </>
  );
}
