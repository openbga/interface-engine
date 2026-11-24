---
date: 2026-11-24
type: changed
impact: medium
title: PDF reports are filed by date inside their outcome folders
area: Endpoints
link: /endpoints
---
When a PDF report channel moves an incoming PDF after reading it, the engine now places the
file inside a `YYYY/MM/DD` date folder (using the host computer's local date when the report
was received) under the configured processed, filtered or unreadable folder — for example
`processed/2026/11/24/report.pdf`. The configured folder names stay the same, and the date
folders are created automatically on both local folders and SFTP servers.

## Technical notes

Applies to `pdf_report` sources with `on_consumed: move` on `folder` and `sftp` endpoints,
using the message's `received_at` converted to the host's local calendar date
(`<outcome_directory>/YYYY/MM/DD`). Delimited file sources are unchanged.
