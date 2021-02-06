import React from "react";
import styled from "styled-components";
import {
  selectPort,
  useDevices,
  usePortOptions,
  useServiceDispatch,
} from "../state-context";
import { Row } from "./common";

const Heading = styled.div`
  display: flex;
  flex-direction: row;
  justify-content: space-between;
`;

export function Header() {
  const devices = useDevices();
  const portOptions = usePortOptions();
  const serviceDispatch = useServiceDispatch();

  function onPortSelectionChanged(choice: number) {
    console.log("Selected " + choice);
    selectPort(serviceDispatch, choice);
  }

  return (
    <Heading>
      <p>
        {devices.length > 0
          ? `Device '${devices[0].device_name}' is connected, you're all set!`
          : "No compatible devices could be found!"}
      </p>
      <Row>
        <p>Output Port:</p>
        {(portOptions?.length ?? 0) > 0 && (
          <select
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
          </select>
        )}
      </Row>
    </Heading>
  );
}
