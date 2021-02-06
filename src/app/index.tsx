import React from "react";
import ReactDOM from "react-dom";
import App from "./App";
import styled from "styled-components";
import { SettingsProvider } from "./settings-context";
import { ServiceStateProvider } from "./state-context";

const Root = styled.div`
  color: white;
  background-color: #282c34;
  min-height: 100vh;
`;

ReactDOM.render(
  <React.StrictMode>
    <Root>
      <ServiceStateProvider>
        <SettingsProvider>
          <App />
        </SettingsProvider>
      </ServiceStateProvider>
    </Root>
  </React.StrictMode>,
  document.getElementById("root")
);
