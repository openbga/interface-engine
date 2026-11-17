---
date: 2026-11-12
type: added
impact: high
title: Transmission reports and message exports
area: Transmission report
link: /transmission
---
Show, for a period, what each destination received and how it acknowledged it, and save the report
as CSV or JSON. Messages can also be exported with filters. Both are recorded in the audit log, and
their files can contain patient data.

## Technical notes

`packie messages verify` produces per-destination transmission summaries and CSV/JSON reports with
original/transformed hashes, ACK level/code and the ACK's own control id; `packie messages export`
writes filtered CSV/JSON Lines with optional original bodies. Both are attributed and audited;
outputs can contain PHI.
