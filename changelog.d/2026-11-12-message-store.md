---
date: 2026-11-12
type: added
impact: high
title: Every message is stored and can be searched
area: Messages
link: /messages
release: 0.1.0
---
The engine keeps each message as received and as delivered, with every delivery attempt, and you can
search them. Old messages are removed on a schedule, never while work on them is unfinished.

## Technical notes

Bundled SQLite WAL store with one writer, original and transformed bodies, delivery attempts,
message search, retention that preserves unresolved work, and migrations. If the store cannot be
opened the engine starts degraded, keeps bridges running and retries.
