---
date: 2026-11-12
type: fixed
impact: low
title: Configuration fixes
area: Settings
link: /settings/versions
---
Signing off a verification no longer crashes, a configuration with extra text after it is rejected,
and retry multipliers keep their exact value when saved.

## Technical notes

Verification sign-off no longer crashes; validation rejects trailing JSON text, and retry
multipliers retain their precision when configuration is saved.
