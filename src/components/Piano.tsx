import React from "react";
import styled from "styled-components";
import { PianoDisplay, MidiDataEntry } from "./PianoDisplay";

const PianoHolder = styled.div`
  width: 90%;
  height: 80px;
  padding: 1em;
`;

interface Props {
  setIsMousePressed: (value: number | null) => void;
  setNoteMapping: (midiNumber: number) => void;
  pianoData: MidiDataEntry[];
}

export function Piano(props: Props) {
  return (
    <PianoHolder
      onMouseDown={(event) => {
        // event.stopPropagation();
        props.setIsMousePressed(event.button);
        setTimeout(() => {
          props.setIsMousePressed(null);
        }, 3000);
      }}
    >
      <PianoDisplay
        midiData={props.pianoData}
        changeMidiMap={props.setNoteMapping}
      />
    </PianoHolder>
  );
}
