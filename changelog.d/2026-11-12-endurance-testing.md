---
date: 2026-11-12
type: added
impact: low
title: Long-running endurance and fuzz testing
area: Engine
---
The engine was run for hours under crashes, full storage, destination outages and configuration
changes without losing or duplicating a message, and its message readers are fuzz-tested every week.

## Technical notes

Chaos soak harness covering crashes, blocked storage, destination outages and configuration
changes. A one-hour run handled 64,870 messages and an eight-hour run 565,791; after the two
defects the long run exposed were fixed, a four-hour run handled 284,941. No run lost or
duplicated a message. Twelve fuzz targets each ran clean for at least ten minutes after an
allocation defect was fixed, with weekly fuzz runs configured in CI.
