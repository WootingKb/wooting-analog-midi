import { floor } from "lodash";

const OCTAVE_NOTE_NO = 12;

const baseNames: (string | ((flat: boolean) => string))[] = [
  "C",
  (flat) => (flat ? "Db" : "C#"),
  "D",
  (flat) => (flat ? "Eb" : "D#"),
  "E",
  "F",
  (flat) => (flat ? "Gb" : "F#"),
  "G",
  (flat) => (flat ? "Ab" : "G#"),
  "A",
  (flat) => (flat ? "Bb" : "A#"),
  "B",
];

export function midiNumberToNote(noteID: number, flat: boolean = false): string {
  const octaveNumber = floor(noteID / OCTAVE_NOTE_NO) - 1;
  const octavePart = noteID % OCTAVE_NOTE_NO;
  let baseName = baseNames[octavePart];
  if (typeof baseName == "function") {
    baseName = baseName(flat);
  }

  return `${baseName}${octaveNumber}`;
}
