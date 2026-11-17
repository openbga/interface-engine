---
date: 2026-11-12
type: changed
impact: medium
title: Messages are stored and delivered several times faster
area: Engine
---
The engine stores about 2,450 messages a second across 50 channels, up from 525, and each message is
saved in well under a tenth of a second.

## Technical notes

Measured durable throughput improved from 525 to about 2,450 messages/s across 50 channels, with
commit p99 reduced from 130–293 ms to 43–47 ms. One-channel throughput improved from 467 to about
1,250 messages/s; idle CPU measured 1.8 % of one core.
