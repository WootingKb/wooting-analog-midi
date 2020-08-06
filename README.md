# Wooting Analog MIDI

Experimental cross-platform Rust implementation for a Virtual MIDI device using the [Wooting Analog SDK](https://github.com/WootingKb/wooting-analog-sdk)!

## Features

- [x] Virtual MIDI Input from Wooting Analog Keyboards
- [x] Velocity Input
- [x] Polyphonic Aftertouch
- [x] Interactively bind keys to MIDI notes per channel (left click bind, right click unbind)
- [x] Channel Selection
- [x] Shift key to shift Octave
- [ ] Channel Aftertouch

## Project Status

This project began as a side-project and is currently at a MVP (Minimum viable product) stage. We want to hear from you if this is something you'd like us to develop more. You're also welcome to contribute to the project if you desire. Feedback, testing/bug reporting and code contributions would all be greatly appreciated!

## Getting Started

### Downloads

Downloads for each platform can be found on the [latest release](https://github.com/WootingKb/wooting-analog-midi/releases/latest)

### Setup

#### Mac

You may need to follow [this guide](https://medium.com/@keybaudio/virtual-midi-devices-on-macos-a45cdbdffdaf) to create a virtual MIDI device for the Application to output to!

## Development Setup

This section is only relevant if you wish to help develop/contribute code to the project!

### Dependencies

- [yarn](https://yarnpkg.com/getting-started) Is our preferred Node package manager
- [Rust & Tauri](https://tauri.studio/docs/getting-started/intro#setting-up-your-environment)

#### Linux

The `libasound2-dev` package may be required to be installed:

```bash
sudo apt install libasound2-dev
```

For packaging `AppImage` `squashfs-tools` may be required:

```bash
sudo apt install squashfs-tools
```

### Directory Structure

- `src` - React Frontend source code
- `wooting-analog-midi` - Rust source for the virtual MIDI device using the [Wooting Analog SDK](https://github.com/WootingKb/wooting-analog-sdk)!
- `src-tauri` - The Tauri host process code which bootstraps the web view & contains the glue code between the React frontend and the Rust backend

### Get going

First you gotta install dependencies of the project

```bash
yarn
```

To help with development it's useful to export the `RUST_LOG` environment variable to get more debugging output from the application
e.g.

```bash
# Bash
## To have it for your entire terminal session
export RUST_LOG=debug
## Or to have it just for the dev command
RUST_LOG=debug yarn tauri dev


# Powershell
$env:RUST_LOG="debug"

# CMD
set RUST_LOG=debug
```

Then you should be able to run the application in development mode, which includes hot reloading automatically on save:

```bash
yarn tauri dev
```

If you want to build a distributable binary/package run:

```bash
yarn tauri build
```

For more details & other commands, Tauri has a good reference for [development commands here](https://tauri.studio/docs/usage/development/development)
