---
date: 2026-11-12
type: added
impact: medium
title: Compare two configuration versions field by field
area: Settings
link: /settings/versions
---
See exactly what differs between two saved versions, the active configuration or a configuration
file before you apply it.

## Technical notes

`packie config diff` compares saved versions, active configuration and runtime files field by
field; `packie config export` aliases the active JSON export.
