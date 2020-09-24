import * as React from "react";
import { useEffect } from "react";
import { AppSettings, backend } from "./backend";
import { HIDCodes } from "./HidCodes";
type InitAction<S> = { type: "INIT"; value: S };

type SettingsAction =
  | { type: "change"; settings: AppSettings }
  | { type: "CHANGE_MAPPING"; mapping: [HIDCodes, number][]; channel: number }
  | { type: "NOTE_SHIFT_CHANGED"; value: number }
  | { type: "THRESHOLD_CHANGED"; value: number }
  | { type: "VELOCITY_SCALE_CHANGED"; value: number }
  | InitAction<AppSettings>;
export type SettingsDispatch = (action: SettingsAction) => void;
type SettingsState = AppSettings;
type SettingsProviderProps = { children: React.ReactNode };
const SettingsStateContext = React.createContext<SettingsState | undefined>(
  undefined
);
const SettingsDispatchContext = React.createContext<
  SettingsDispatch | undefined
>(undefined);

function settingsReducer(
  state: SettingsState,
  action: SettingsAction
): SettingsState {
  switch (action.type) {
    case "change":
      return action.settings;
    case "NOTE_SHIFT_CHANGED":
      return { ...state, shift_amount: action.value };
    case "CHANGE_MAPPING":
      return {
        ...state,
        keymapping: {
          ...state.keymapping,
          [action.channel]: action.mapping,
        },
      };
    case "THRESHOLD_CHANGED":
      return {
        ...state,
        note_config: {
          ...state.note_config,
          threshold: action.value,
        },
      };
    case "VELOCITY_SCALE_CHANGED":
      return {
        ...state,
        note_config: { ...state.note_config, velocity_scale: action.value },
      };
    default: {
      //@ts-ignore
      console.error(`Unhandled action type: ${action.type}`);
      return state;
    }
  }
}

function isInitAction<S>(action: any): action is InitAction<S> {
  return "type" in action && action.type === "INIT";
}

function undefinedReducer<S, A extends { type: string } | InitAction<S>>(
  reducer: React.Reducer<S, A>
): React.Reducer<S | undefined, A> {
  return (state, action) => {
    if (isInitAction<S>(action)) {
      return action.value;
    } else if (state) {
      return reducer(state, action);
    }
  };
}

function SettingsProvider({ children }: SettingsProviderProps) {
  const [state, dispatch] = React.useReducer(
    undefinedReducer(settingsReducer),
    undefined
  );

  useEffect(() => {
    if (state) {
      backend.updateSettings(state);
    }
  }, [state]);

  useEffect(() => {
    backend.setSettingsDispatcher(dispatch);
  }, [dispatch]);
  // console.log(state);
  return state ? (
    <SettingsStateContext.Provider value={state}>
      <SettingsDispatchContext.Provider value={dispatch}>
        {children}
      </SettingsDispatchContext.Provider>
    </SettingsStateContext.Provider>
  ) : (
    <div />
  );
}

function useSettingsState() {
  const context = React.useContext(SettingsStateContext);
  if (context === undefined) {
    throw new Error("useCountState must be used within a CountProvider");
  }
  return context;
}
function useSettingsDispatch() {
  const context = React.useContext(SettingsDispatchContext);
  if (context === undefined) {
    throw new Error("useCountDispatch must be used within a CountProvider");
  }
  return context;
}

function useSettings(): [SettingsState, SettingsDispatch] {
  return [useSettingsState(), useSettingsDispatch()];
}

export { SettingsProvider, useSettingsState, useSettingsDispatch, useSettings };
