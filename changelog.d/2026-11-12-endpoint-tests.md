---
date: 2026-11-12
type: added
impact: medium
title: Find serial ports and test an endpoint before you rely on it
area: Endpoints
link: /endpoints
---
The engine lists the serial ports this computer has, with their USB details and what already uses
them. Test an endpoint's settings, saved or not, and see each step's result without sending any
data.

## Technical notes

`packie endpoints list-serial` shows OS ports, USB details and configured use; `packie endpoints
test` checks saved or unsaved serial, TCP, folder and SFTP settings without sending application
data, with per-step results and `endpoint_test` audit records.
