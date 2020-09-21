import React from "react";
import styled from "styled-components";
import { Column } from "./common";
import { MIDI_NOTE_MAX } from "../backend";
import { useSettings } from "../settings-context";

const SettingsBody = styled.div`
  display: flex;
  flex-direction: row;
  flex-wrap: wrap;
`;

const NumberInput = styled.input`
  text-align: center;
`;

export function Settings() {
  const [appSettings, appSettingsDispatch] = useSettings();

  return (
    <SettingsBody>
      <Column>
        <p>Shift Amount</p>

        <NumberInput
          type="number"
          value={appSettings.shift_amount}
          onChange={(event) => {
            const value = parseInt(event.target.value);
            if (value !== appSettings.shift_amount) {
              appSettingsDispatch({
                type: "NOTE_SHIFT_CHANGED",
                value,
              });
            }
          }}
          min={-MIDI_NOTE_MAX}
          max={MIDI_NOTE_MAX}
        />
      </Column>
      <Column>
        <p>Note Trigger Threshold</p>

        <NumberInput
          type="number"
          value={appSettings.note_config.threshold.toPrecision(2)}
          onChange={(event) => {
            const value = parseFloat(event.target.value);
            if (value !== appSettings.shift_amount) {
              appSettingsDispatch({
                type: "THRESHOLD_CHANGED",
                value,
              });
            }
          }}
          min={0}
          max={1}
          step={0.1}
        />
      </Column>
    </SettingsBody>
  );
}
