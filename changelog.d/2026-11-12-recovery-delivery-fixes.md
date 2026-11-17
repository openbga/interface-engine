---
date: 2026-11-12
type: fixed
impact: medium
title: More reliable recovery and delivery
area: Channels
link: /channels
---
Fixed problems with shutting down during startup, back-to-back ASTM transfers, HL7 sessions that
stop answering and recovery after a restart. Protocol settings and acknowledgement rules now hold
through delivery and recovery.

## Technical notes

Early shutdown during startup, consecutive ASTM transfers, unresponsive HL7 sessions, retention
races and retry of startup reconciliation. Protocol settings and acknowledgement policies remain
consistent through delivery and recovery.
