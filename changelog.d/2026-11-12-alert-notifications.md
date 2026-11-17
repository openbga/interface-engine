---
date: 2026-11-12
type: added
impact: high
title: Alerts by e-mail and webhook
area: Settings
link: /settings/notifications
---
The engine can send its alerts by e-mail or to a webhook, choosing which severities go where.
Settings, Notifications lists the targets and sends a test message; notifications never contain
patient data.

## Technical notes

Bootstrap SMTP and webhook alert targets with severity selection, shared rate limiting, PHI-free
operational content, secret references and a private-CA PEM bundle; `packie notify test`, audited
as `notification_test`, and `notifications_failing` for failed deliveries.
