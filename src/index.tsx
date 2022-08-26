import React from "react";
import ReactDOM from "react-dom";
import "./index.css";
import App from "./App";
import { SettingsProvider } from "./settings-context";
import { ServiceStateProvider } from "./state-context";
import { Box, ChakraProvider } from "@chakra-ui/react";

ReactDOM.render(
  <React.StrictMode>
    <ChakraProvider>
      <Box minH="100vh">
        <ServiceStateProvider>
          <SettingsProvider>
            <App />
          </SettingsProvider>
        </ServiceStateProvider>
      </Box>
    </ChakraProvider>
  </React.StrictMode>,
  document.getElementById("root")
);
