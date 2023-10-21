import React from "react";
import { MIDI_NOTE_MAX } from "../backend";
import { useSettings } from "../settings-context";
import {
  NumberInput,
  NumberInputField,
  NumberInputStepper,
  NumberIncrementStepper,
  NumberDecrementStepper,
  Text,
  HStack,
  VStack,
} from "@chakra-ui/react";

export function Settings() {
  const [appSettings, appSettingsDispatch] = useSettings();

  return (
    <HStack flexWrap="wrap" justifyContent="space-evenly">
      <VStack>
        <Text>Shift Amount</Text>

        <NumberInput
          value={appSettings.shift_amount}
          min={-MIDI_NOTE_MAX}
          max={MIDI_NOTE_MAX}
          onChange={(_, value) => {
            if (!isNaN(value) && value !== appSettings.shift_amount) {
              appSettingsDispatch({
                type: "NOTE_SHIFT_CHANGED",
                value,
              });
            }
          }}
        >
          <NumberInputField />
          <NumberInputStepper>
            <NumberIncrementStepper />
            <NumberDecrementStepper />
          </NumberInputStepper>
        </NumberInput>
      </VStack>
      <VStack>
        <Text>Note Trigger Threshold</Text>

        <NumberInput
          value={appSettings.note_config.threshold.toPrecision(2)}
          onChange={(_, value) => {
            if (!isNaN(value) && value !== appSettings.shift_amount) {
              appSettingsDispatch({
                type: "THRESHOLD_CHANGED",
                value,
              });
            }
          }}
          min={0}
          max={1}
          step={0.01}
        >
          <NumberInputField />
          <NumberInputStepper>
            <NumberIncrementStepper />
            <NumberDecrementStepper />
          </NumberInputStepper>
        </NumberInput>
      </VStack>
      <VStack>
        <Text>Velocity Scale</Text>

        <NumberInput
          value={appSettings.note_config.velocity_scale}
          onChange={(_, value) => {
            if (!isNaN(value) && value !== appSettings.shift_amount) {
              appSettingsDispatch({
                type: "VELOCITY_SCALE_CHANGED",
                value,
              });
            }
          }}
          min={0.1}
          max={20}
          step={0.1}
        >
          <NumberInputField />
          <NumberInputStepper>
            <NumberIncrementStepper />
            <NumberDecrementStepper />
          </NumberInputStepper>
        </NumberInput>
      </VStack>
    </HStack>
  );
}
