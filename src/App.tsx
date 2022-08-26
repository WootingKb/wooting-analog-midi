import "core-js";
import React from "react";
import { PianoBody } from "./components/Piano";
import { MIDIStateDisplay } from "./components/MIDIStateDisplay";
import { Settings } from "./components/Settings";
import { Header } from "./components/Header";
import { Box, Flex } from "@chakra-ui/react";

function App() {
  return (
    <Box p="1em">
      <Header />
      <Flex direction="column" align="center">
        <PianoBody />
      </Flex>
      <Settings />
      <MIDIStateDisplay />
    </Box>
  );
}

export default App;
