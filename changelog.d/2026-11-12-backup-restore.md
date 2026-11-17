---
date: 2026-11-12
type: added
impact: high
title: Back up and restore the engine's data
area: Settings
link: /settings/backup
---
You can back up the message store, the audit log and the configuration in one consistent copy, by
hand or on a daily schedule, and check or restore a backup later. Settings, Backup shows the last
backup and offers Back up now; restore runs from the command line with the engine stopped.

## Technical notes

`packie backup create`, `verify`, `restore` and `status`; consistent store/body, audit and
configuration snapshots with SHA-256 manifests, optional daily scheduling, backup-count retention
and `backup_stale` alerts. Restore requires a stopped engine and preserves replaced state in a
pre-restore directory; secret values are excluded.
