---
date: 2026-11-12
type: changed
impact: medium
title: Resending an older revision of a message is refused unless you confirm it
area: Messages
link: /messages
---
When a newer revision of a message exists, resending the older one is refused by default, from the
console and the command line. You can still override it explicitly, and the override is recorded in
the audit log.

## Technical notes

Resending a superseded revision is refused by default in both modes. The explicit
`--superseded-ok` override is audited and remains separate from duplicate-risk confirmation.
