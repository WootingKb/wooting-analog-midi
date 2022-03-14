import { MoonIcon, SunIcon } from "@chakra-ui/icons";
import {
  HStack,
  IconButton,
  Select,
  Text,
  useColorMode,
} from "@chakra-ui/react";
import React from "react";
import {
  selectPort,
  useDevices,
  usePortOptions,
  useServiceDispatch,
} from "../state-context";

export function Header() {
  const devices = useDevices();
  const portOptions = usePortOptions();
  const serviceDispatch = useServiceDispatch();

  function onPortSelectionChanged(choice: number) {
    console.log("Selected " + choice);
    selectPort(serviceDispatch, choice);
  }
  const { colorMode, toggleColorMode } = useColorMode();

  return (
    <HStack justifyContent="space-between">
      <Text>
        {devices.length > 0
          ? `Connected Devices: ${devices.map((d) => d.device_name).join(", ")}`
          : "No compatible devices could be found!"}
      </Text>
      <HStack>
        <IconButton
          variant="ghost"
          aria-label="Color Mode"
          onClick={toggleColorMode}
          icon={colorMode === "light" ? <MoonIcon /> : <SunIcon />}
        />
        <Text minW="max-content">Output Port:</Text>
        {(portOptions?.length ?? 0) > 0 && (
          <Select
            value={portOptions.findIndex((item) => item[2])}
            onChange={(event) => {
              onPortSelectionChanged(parseInt(event.target.value));
            }}
          >
            {portOptions.map((item) => (
              <option key={item[0]} value={item[0]}>
                {item[1]}
              </option>
            ))}
          </Select>
        )}
      </HStack>
    </HStack>
  );
}
