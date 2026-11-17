---
date: 2026-11-12
type: added
impact: high
title: Command line for running and managing the engine
area: Command line
release: 0.1.0
---
The `packie` command starts the engine, shows its status, pauses and restarts channels, finds and
resends messages, follows the logs, applies configuration, stores secrets and installs the service.
Every command can also answer in JSON for scripts.

## Technical notes

`packie run`, `status`, `channels` (pause, resume, restart), `messages` (list, show, resend,
close), `logs` (with `--follow`), `config` (show, validate, apply, history), `secrets` and
`service` (install, uninstall, start, stop, status), with `--json` output and exit codes for
scripts.
