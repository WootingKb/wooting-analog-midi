import "core-js";
import React from "react";
import styled from "styled-components";
import { PianoBody } from "./components/Piano";
import { MIDIStateDisplay } from "./components/MIDIStateDisplay";
import { Settings } from "./components/Settings";
import { Header } from "./components/Header";

const AppRoot = styled.div`
  padding: 1em;
`;

const Body = styled.div`
  display: flex;
  flex-direction: column;
  align-items: center;
`;

function App() {
  return (
    <AppRoot>
      <Header />
      <Body>
        <PianoBody />
      </Body>
      <Settings />
      <MIDIStateDisplay />
    </AppRoot>
  );
}

export default App;
