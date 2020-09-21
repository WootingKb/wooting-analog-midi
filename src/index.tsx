import React from "react";
import ReactDOM from "react-dom";
import "./index.css";
import App from "./App";
import styled from "styled-components";
import { SettingsProvider } from "./settings-context";

const Root = styled.div`
  color: white;
  background-color: #282c34;
  min-height: 100vh;
`;

ReactDOM.render(
  <React.StrictMode>
    <Root>
      <SettingsProvider>
        <App />
      </SettingsProvider>
    </Root>
  </React.StrictMode>,
  document.getElementById("root")
);
