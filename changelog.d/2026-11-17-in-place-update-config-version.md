---
date: 2026-11-17
type: fixed
impact: medium
title: Messages show the configuration version in effect after an in-place change
area: Channels
link: /channels
pr: 5
---
After a channel change that applied without a restart, new messages and the channel's status kept
showing the version the channel started with. They now show the version in effect, matching their
deliveries.

## Technical notes

New messages, the channel snapshot and bridge tap observations read the configuration version of
the definition in effect; the snapshot's `config_version` follows in-place updates.
