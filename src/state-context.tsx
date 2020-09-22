import * as React from "react";
import { useEffect } from "react";
import { backend, DeviceList, MidiUpdate, PortOptions } from "./backend";
import {
  createContext,
  useContextSelector,
  useContext,
} from "use-context-selector";
import _ from "lodash";

export type ServiceStateAction =
  | { type: "MIDI_UPDATE"; value: MidiUpdate }
  | { type: "PORT_OPTIONS"; value: PortOptions }
  | { type: "FOUND_DEVICES"; value: DeviceList }
  | { type: "NO_DEVICES" };
export type ServiceStateDispatch = (action: ServiceStateAction) => void;
export interface ServiceStateState {
  midiState: MidiUpdate;
  portOptions: PortOptions;
  connectedDevices: DeviceList;
}
type ServiceStateProviderProps = { children: React.ReactNode };
const ServiceStateStateContext = createContext<ServiceStateState | undefined>(
  undefined
);
const ServiceStateDispatchContext = createContext<
  ServiceStateDispatch | undefined
>(undefined);

function serviceStateReducer(
  state: ServiceStateState,
  action: ServiceStateAction
): ServiceStateState {
  // console.log(action);
  switch (action.type) {
    case "MIDI_UPDATE":
      if (_.isEqual(action.value, state.midiState)) {
        return state;
      } else {
        return { ...state, midiState: action.value };
      }
    case "PORT_OPTIONS":
      return { ...state, portOptions: action.value };
    case "NO_DEVICES":
      return { ...state, connectedDevices: [] };
    case "FOUND_DEVICES":
      return { ...state, connectedDevices: action.value };
    default: {
      //@ts-ignore
      console.error(`Unhandled action type: ${action.type}`);
      return state;
    }
  }
}

function ServiceStateProvider({ children }: ServiceStateProviderProps) {
  const [state, dispatch] = React.useReducer(serviceStateReducer, {
    midiState: { data: {} },
    portOptions: [],
    connectedDevices: [],
  });

  // useEffect(() => {
  //   if (state) {
  //     backend.updateSettings(state);
  //   }
  // }, [state]);

  useEffect(() => {
    backend.setServiceDispatcher(dispatch);
  }, [dispatch]);
  // console.log(state);
  return state ? (
    <ServiceStateStateContext.Provider value={state}>
      <ServiceStateDispatchContext.Provider value={dispatch}>
        {children}
      </ServiceStateDispatchContext.Provider>
    </ServiceStateStateContext.Provider>
  ) : (
    <div />
  );
}

function useServiceState() {
  const context = useContext(ServiceStateStateContext);
  if (context === undefined) {
    throw new Error("useCountState must be used within a CountProvider");
  }
  return context;
}
function useServiceDispatch() {
  const context = useContext(ServiceStateDispatchContext);
  if (context === undefined) {
    throw new Error("useCountDispatch must be used within a CountProvider");
  }
  return context;
}

function useService(): [ServiceStateState, ServiceStateDispatch] {
  return [useServiceState(), useServiceDispatch()];
}

function useServiceSelector<S>(selector: (value: ServiceStateState) => S) {
  const context = useContext(ServiceStateDispatchContext);
  if (context === undefined) {
    throw new Error("useCountDispatch must be used within a CountProvider");
  }

  //@ts-ignore
  return useContextSelector(ServiceStateStateContext, selector);
}

function usePortOptions(): PortOptions {
  return useServiceSelector((state) => state.portOptions);
}

function useMidiState(): MidiUpdate {
  return useServiceSelector((state) => state.midiState);
}

function useDevices(): DeviceList {
  return useServiceSelector((state) => state.connectedDevices);
}

function selectPort(dispatch: ServiceStateDispatch, option: number) {
  backend.selectPort(option).then((ports) => {
    dispatch({ type: "PORT_OPTIONS", value: ports });
  });
}

export {
  ServiceStateProvider,
  useServiceState,
  useServiceDispatch,
  useService,
  useServiceSelector,
  selectPort,
  usePortOptions,
  useMidiState,
  useDevices,
};
