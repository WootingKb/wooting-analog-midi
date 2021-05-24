import React from "react";
import ReactDOM from "react-dom";
import "./index.css";
import App from "./App";
import styled from "styled-components";
import { SettingsProvider } from "./settings-context";
import { ServiceStateProvider } from "./state-context";
import { ChakraProvider } from "@chakra-ui/react";

const Root = styled.div`
  // color: white;
  // background-color: #282c34;
  min-height: 100vh;
`;

ReactDOM.render(
  <React.StrictMode>
    <ChakraProvider>
      <Root>
        <ServiceStateProvider>
          <SettingsProvider>
            <App />
          </SettingsProvider>
        </ServiceStateProvider>
      </Root>
    </ChakraProvider>
  </React.StrictMode>,
  document.getElementById("root")
);
