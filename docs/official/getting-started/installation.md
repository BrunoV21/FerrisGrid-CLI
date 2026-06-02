# Installation

FerrisGrid is currently installed from a local checkout.

```bash
git clone https://github.com/BrunoV21/FerrisPilot.git
cd FerrisPilot
cargo build
```

Run the CLI through Cargo:

```bash
cargo run -q -p ferrisgrid-cli -- doctor
```

## Environment variables

| Variable | Purpose |
| --- | --- |
| `FERRISGRID_BACKEND` | Selects the capture/input backend when supported. |
| `FERRISGRID_OUTPUT_DIR` | Changes where `.ferrisgrid` session data is written. |
| `FERRISGRID_DEFAULT_SCREEN_ID` | Sets a default screen target for observe/action contexts. |
| `FERRISGRID_MAX_IMAGE_EDGE` | Sets a fixed default maximum screenshot edge, or `native` to disable downsampling. Leave unset for the adaptive `balanced` default. |

## Docker image

The Linux workspace image installs the CLI inside the container. Use it when you want FerrisGrid to control a background desktop instead of your visible machine.
