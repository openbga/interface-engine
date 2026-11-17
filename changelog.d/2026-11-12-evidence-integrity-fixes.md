---
date: 2026-11-12
type: fixed
impact: medium
title: Backups, exports and reports are more dependable
---
Fixed problems in backup restore, exports that continue over several runs, the order of the
configuration history and the totals of transmission reports. File output keeps to its size limit,
and endpoint tests no longer take a serial port another channel holds.

## Technical notes

Transactional backup restore, exclusion of secret values from backups, audited export
continuations, configuration-history ordering and transmission report totals/truncation.
Delimited-file output respects its configured size limit, and endpoint checks respect exclusive
serial-port ownership.
