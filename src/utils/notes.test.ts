import { midiNumberToNote } from "./notes";

describe("midNumberToNote testing", () => {
  test("Various notes Sharp", () => {
    expect(midiNumberToNote(12)).toEqual("C0");
    expect(midiNumberToNote(24)).toEqual("C1");

    expect(midiNumberToNote(15)).toEqual("D#0");
    expect(midiNumberToNote(89)).toEqual("F6");
    // expect(midiNumberToNote(12)).toEqual("C0");
    // expect(midiNumberToNote(12)).toEqual("C0");
  });

  test("Various notes flat", () => {
    expect(midiNumberToNote(12, true)).toEqual("C0");
    expect(midiNumberToNote(24, true)).toEqual("C1");

    expect(midiNumberToNote(15, true)).toEqual("Eb0");
    expect(midiNumberToNote(89, true)).toEqual("F6");
  });
});
