---
date: 2026-11-12
type: added
impact: high
title: The engine runs as a service on Windows, macOS and Linux
area: Engine
link: /dashboard
release: 0.1.0
---
The engine runs in the background as an operating-system service, or in a terminal, and the console
and the command line manage it from the same computer. It keeps logs and raises alerts for
endpoints, deliveries, storage, disk space and configuration.

## Technical notes

Rust engine that runs as an OS service (Windows service, launchd, systemd) or in a console, one
instance per state directory. Local management over a Unix socket or Windows named pipe, validated
and versioned configuration applied per channel, structured logs with a bounded in-memory ring,
snapshots, and alerts for endpoint, delivery, storage, disk and configuration conditions.
