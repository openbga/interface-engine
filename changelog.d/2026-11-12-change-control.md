---
date: 2026-11-12
type: added
impact: high
title: Configuration changes carry a note, an author and a verification sign-off
area: Settings
link: /settings/versions
---
Applying a configuration now records who applied it and why, and its verification can be signed
off with the account, the time and a reference. Settings, Versions shows each version with these
records, and every version is kept unless you set a limit.

## Technical notes

Attributed configuration records, required apply notes, verification sign-offs with
account/time/reference and `packie config verify`; versions retained without limit by default,
with audit-retention protection when count pruning is configured.
