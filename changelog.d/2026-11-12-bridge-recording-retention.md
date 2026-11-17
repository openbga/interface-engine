---
date: 2026-11-12
type: changed
impact: medium
title: Bridge recordings are kept for 30 days by default
area: Channels
link: /channels
---
Recorded bridge traffic is now removed after 30 days unless you set another limit, so recordings no
longer grow without bound.

## Technical notes

Bridge recordings use `tap.retention`, defaulting to 30 days with no count limit. The retention
run purges observations beyond either limit; `retention_unbounded` warns only when recording is
enabled and neither limit is set.
