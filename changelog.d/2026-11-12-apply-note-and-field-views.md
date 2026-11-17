---
date: 2026-11-12
type: changed
impact: high
title: Applying a configuration needs a note, and field views are recorded
area: Settings
link: /settings/versions
---
Every configuration you apply now needs a short note saying why, and the version history shows who
applied it and its verification. Viewing a message's parsed fields is a separate, recorded action.

## Technical notes

`packie config apply` requires `--note`; history includes applied-by and verification records.
Original parsed fields are requested explicitly with `packie messages show --fields` and the view
is audited.
