---
date: 2026-11-23
type: security
impact: high
title: Safer e-mail notifications
area: Settings
link: /settings/notifications
---
The component that sends alert e-mails was updated to fix reported security problems. A hostile
or compromised mail server could fake replies, exhaust the engine's memory or interrupt sending,
and some addresses could receive a message meant for another. No setting changes; notifications
work as before.

## Technical notes

`lettre` 0.11.23 → 0.11.24 (RUSTSEC-2026-0560 to RUSTSEC-2026-0565) and `url` 2.5.8 → 2.5.9
(RUSTSEC-2026-0592), in the engine and the console's lock files. The workspace now requires
`lettre` 0.11.24 or later.
