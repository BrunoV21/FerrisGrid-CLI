# Installation

For normal use, install the published CLI with Cargo:

```bash
cargo install ferrisgrid-cli
ferrisgrid doctor
```

The crates.io package is `ferrisgrid-cli`; the installed command is `ferrisgrid`.

## TypeScript npm mirror

An equivalent TypeScript package is maintained at [`BrunoV21/FerrisGrid-CLI-ts`](https://github.com/BrunoV21/FerrisGrid-CLI-ts). It mirrors the Rust CLI behavior for Node.js distribution; new features and protocol changes should still be requested in [`BrunoV21/FerrisGrid-CLI`](https://github.com/BrunoV21/FerrisGrid-CLI).

```bash
npm install -g ferrisgrid-cli
ferrisgrid doctor
```

## Development from source

Use a local checkout when you want to build, test, or modify FerrisGrid:

```bash
git clone https://github.com/BrunoV21/FerrisGrid-CLI.git
cd FerrisGrid-CLI
cargo build
cargo test --workspace
```

Run the CLI through Cargo:

```bash
cargo run -q -p ferrisgrid-cli -- doctor
```

## macOS permissions

Normal native capture and demonstration recording require Screen Recording access.
Native input and the global demonstration event tap require Accessibility/Input
Monitoring access. Grant the terminal or agent application running FerrisGrid access
in **System Settings > Privacy & Security**, restart that application, then run:

```bash
ferrisgrid doctor
```

The native `record` command requires macOS 13 or newer. FerrisGrid prints a three-second
countdown by default so you can focus the application you want to demonstrate.

## Windows

FerrisGrid supports Windows 10/11 x64 from an interactive desktop session. The default `native` backend selects the Win32 capture and input implementation; `native-windows`, `windows`, and `win32` select it explicitly.

Windows has no Screen Recording prompt, but input to elevated applications and the secure desktop can be blocked. Run FerrisGrid and the application being controlled at the same elevation level. ARM64 is not part of the initial Windows release.

## Environment variables

| Variable | Purpose |
| --- | --- |
| `FERRISGRID_BACKEND` | Selects the capture/input backend when supported. |
| `FERRISGRID_OUTPUT_DIR` | Changes where `.ferrisgrid` session data is written. |
| `FERRISGRID_DEFAULT_SCREEN_ID` | Sets a default screen target for observe/action contexts. |
| `FERRISGRID_MAX_IMAGE_EDGE` | Sets a fixed default maximum screenshot edge, or `native` to disable downsampling. Leave unset for the adaptive `balanced` default. |

## Docker image

The Linux workspace image installs the CLI inside the container. Use it when you want FerrisGrid to control a background desktop instead of your visible machine.
