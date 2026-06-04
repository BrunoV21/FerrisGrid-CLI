# FerrisGrid Raw Docs for Agents

This page is a raw Markdown index for agents. Prefer these links when you need documentation without rendered HTML, navigation scripts, or theme markup.

## Primary Entry Points

- [Getting started](https://raw.githubusercontent.com/BrunoV21/FerrisGrid-CLI/main/docs/official/getting-started/index.md)
- [First run](https://raw.githubusercontent.com/BrunoV21/FerrisGrid-CLI/main/docs/official/getting-started/first-run.md)
- [Installation](https://raw.githubusercontent.com/BrunoV21/FerrisGrid-CLI/main/docs/official/getting-started/installation.md)
- [Agent protocol](https://raw.githubusercontent.com/BrunoV21/FerrisGrid-CLI/main/docs/official/concepts/agent-protocol.md)
- [Commands overview](https://raw.githubusercontent.com/BrunoV21/FerrisGrid-CLI/main/docs/official/commands/index.md)

## Command Reference

- [observe](https://raw.githubusercontent.com/BrunoV21/FerrisGrid-CLI/main/docs/official/commands/observe.md)
- [act](https://raw.githubusercontent.com/BrunoV21/FerrisGrid-CLI/main/docs/official/commands/act.md)
- [doctor](https://raw.githubusercontent.com/BrunoV21/FerrisGrid-CLI/main/docs/official/commands/doctor.md)
- [recap](https://raw.githubusercontent.com/BrunoV21/FerrisGrid-CLI/main/docs/official/commands/recap.md)
- [clear](https://raw.githubusercontent.com/BrunoV21/FerrisGrid-CLI/main/docs/official/commands/clear.md)

## Concepts

- [Architecture](https://raw.githubusercontent.com/BrunoV21/FerrisGrid-CLI/main/docs/official/concepts/architecture.md)
- [Agent protocol](https://raw.githubusercontent.com/BrunoV21/FerrisGrid-CLI/main/docs/official/concepts/agent-protocol.md)
- [Local traces](https://raw.githubusercontent.com/BrunoV21/FerrisGrid-CLI/main/docs/official/concepts/local-traces.md)

## Workspaces

- [Docker Linux workspace](https://raw.githubusercontent.com/BrunoV21/FerrisGrid-CLI/main/docs/official/workspaces/docker.md)

## Project Docs

- [Community](https://raw.githubusercontent.com/BrunoV21/FerrisGrid-CLI/main/docs/official/community.md)
- [Roadmap](https://raw.githubusercontent.com/BrunoV21/FerrisGrid-CLI/main/docs/official/roadmap.md)
- [Release notes](https://raw.githubusercontent.com/BrunoV21/FerrisGrid-CLI/main/docs/official/releases/index.md)
- [FerrisGrid v0.2.0 release notes](https://raw.githubusercontent.com/BrunoV21/FerrisGrid-CLI/main/docs/official/releases/v0.2.0.md)
- [FerrisGrid v0.1.0 release notes](https://raw.githubusercontent.com/BrunoV21/FerrisGrid-CLI/main/docs/official/releases/v0.1.0.md)

## Recommended Agent Reading Order

1. Read [Agent protocol](https://raw.githubusercontent.com/BrunoV21/FerrisGrid-CLI/main/docs/official/concepts/agent-protocol.md).
2. Read [Commands overview](https://raw.githubusercontent.com/BrunoV21/FerrisGrid-CLI/main/docs/official/commands/index.md).
3. Read the specific command reference for the command you are about to run.
4. Run `ferrisgrid --help` or `ferrisgrid <command> --help` for the installed CLI's local contract.

## Important Runtime Contract

- FerrisGrid is single-step. Alternate `observe` and one `act`.
- Inspect screenshot paths returned by `observe` and `act`.
- Use normalized coordinates from `0..1000`.
- Include `screen_id` for pointer actions on multi-screen systems, or pass `act --screen-id <id>`.
- Use compact Markdown action files. Do not send JSON to `act`.
