---
date: 2026-11-12
type: added
impact: high
title: Installers for Windows, macOS and Linux
area: Installation
release: 0.1.0
---
Each installer puts the engine in place and starts its service. The Windows and macOS installers
include the console; the Linux package has the engine only.

## Technical notes

Windows NSIS installer, macOS package and Linux engine Debian package, each installing and
starting the service; CI checks and a manually dispatched release-build workflow that includes a
reproducible-build check of the engine on Linux and macOS.

Known limitations: installers are unsigned and macOS packages are not notarized. Windows and Linux
installer qualification remains pending; see [packaging/VERIFY.md](packaging/VERIFY.md). The Linux
engine package does not include the console.
