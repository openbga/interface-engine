---
date: 2026-11-12
type: added
impact: high
title: Management stays on the lab computer and secrets stay out of files
area: Engine
release: 0.1.0
---
Only accounts on the engine's own computer that are allowed to manage it can reach it. Passwords
and other secrets are kept in the operating system's key store or a file only the service can
read, never in logs, and listening endpoints accept only the peers you allow.

## Technical notes

Management interface limited to the local computer (socket group `packie`, pipe access control
list); state and log directories private to the service account; secrets in the OS key store or an
owner-only file and never in logs; listener peer allowlists.
