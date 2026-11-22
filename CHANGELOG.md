<!-- Generated from changelog.d/ by `npm run changelog` in console/. Do not edit by hand:
     add or change a file in changelog.d/ instead (CONTRIBUTING.md, Changelog entries). -->

# Changelog

What changed in OpenBGA Interface Engine, grouped by release with unreleased changes on
top, then by impact. Types follow [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## Unreleased (2026-11-12 to 2026-11-22)

### High impact

- **Changed:** The product is now called OpenBGA Interface Engine (2026-11-17, [#3](https://github.com/openbga/interface-engine/pull/3))

  The engine and console carry the new name. Alert e-mail subjects, export files and installers use it
  too, so update any mail rules or scripts that look for the old names. The command, the service and
  the install folders keep their names.

  Technical notes:

  The product is OpenBGA Interface Engine (repository `openbga/interface-engine`, console OpenBGA
  Interface Engine Console). Alert e-mail subjects start `[Interface Engine <node>]`, exports are
  named `interface-engine-*`, installers `openbga-interface-engine-<version>-*`. Install folders,
  the `packie` command, service, group, variables and the Windows group `PACK-IE Operators` keep
  their names (see [install](docs/operations/install.md)).

- **Added:** Alerts by e-mail and webhook (2026-11-12)

  The engine can send its alerts by e-mail or to a webhook, choosing which severities go where.
  Settings, Notifications lists the targets and sends a test message; notifications never contain
  patient data.

  Technical notes:

  Bootstrap SMTP and webhook alert targets with severity selection, shared rate limiting, PHI-free
  operational content, secret references and a private-CA PEM bundle; `packie notify test`, audited
  as `notification_test`, and `notifications_failing` for failed deliveries.

- **Changed:** Applying a configuration needs a note, and field views are recorded (2026-11-12)

  Every configuration you apply now needs a short note saying why, and the version history shows who
  applied it and its verification. Viewing a message's parsed fields is a separate, recorded action.

  Technical notes:

  `packie config apply` requires `--note`; history includes applied-by and verification records.
  Original parsed fields are requested explicitly with `packie messages show --fields` and the view
  is audited.

- **Added:** Audit log of who did what, kept apart from message data (2026-11-12)

  Every operator action, every view of a message body or its fields and every identifier or text
  search is now recorded with the computer account that made it. Open Audit to filter, export and
  verify the log; `packie audit` does the same from the command line.

  Technical notes:

  OS identity from each management connection; separate append-only, hash-chained audit databases
  with yearly sealing, retention, daily/shutdown anchors and `packie audit list`, `export` and
  `verify`. Records cover operator actions, message body/field views and identifier/text searches.
  Audited commands/content views fail closed when their initial record cannot be written; message
  traffic continues. `audit_unavailable` alerts report failures.

- **Added:** Back up and restore the engine's data (2026-11-12)

  You can back up the message store, the audit log and the configuration in one consistent copy, by
  hand or on a daily schedule, and check or restore a backup later. Settings, Backup shows the last
  backup and offers Back up now; restore runs from the command line with the engine stopped.

  Technical notes:

  `packie backup create`, `verify`, `restore` and `status`; consistent store/body, audit and
  configuration snapshots with SHA-256 manifests, optional daily scheduling, backup-count retention
  and `backup_stale` alerts. Restore requires a stopped engine and preserves replaced state in a
  pre-restore directory; secret values are excluded.

- **Added:** Calculations, presets and more protocol settings for instrument channels (2026-11-12)

  Mappings can now calculate exact values, build text, add records and check preconditions, and
  ready-made presets fill in the settings and mappings for common instrument layouts. The channel
  editor shows each preset's values and a preview before you save.

  Technical notes:

  Exact-decimal calculations, text steps, appended records, `require` preconditions and `for_each`
  modes in mappings; six connector presets and seven mapping presets for common instrument layouts,
  with console editors and previews; ASTM `enq_attempts`, `busy_retry_ms`, `sample_id_path` and
  frames up to 64,000 characters; AU `test_code_digits`; HL7 `require_end_cr`, `ack_message_types`
  and `reply_type` for order responses. Specimen identifiers (ASTM O-4, HL7 SPM) reach receipt
  metadata and search.

- **Added:** Configuration changes carry a note, an author and a verification sign-off (2026-11-12)

  Applying a configuration now records who applied it and why, and its verification can be signed
  off with the account, the time and a reference. Settings, Versions shows each version with these
  records, and every version is kept unless you set a limit.

  Technical notes:

  Attributed configuration records, required apply notes, verification sign-offs with
  account/time/reference and `packie config verify`; versions retained without limit by default,
  with audit-retention protection when count pruning is configured.

- **Added:** Message content is hidden when the console is left idle (2026-11-12)

  After a period without activity, 15 minutes unless you change it, the console hides message content
  and searched identifiers until you return and choose to show them again. Set the period in Settings,
  Privacy.

  Technical notes:

  Idle protection conceals message content and searched identifiers until the operator returns.

- **Changed:** On Windows, members of PACK-IE Operators can manage the engine (2026-11-12)

  Besides administrators, members of the local PACK-IE Operators group can now use the console and the
  command line on Windows. Installing the service creates the group and adds the installing account;
  add other operators to it.

  Technical notes:

  Windows management access now uses the local `PACK-IE Operators` group besides SYSTEM and
  Administrators. Default `ipc.group: "packie"` selects this group; service installation creates it
  and adds the installing account. Unix continues to use the `packie` group by default.

- **Added:** Transmission reports and message exports (2026-11-12)

  Show, for a period, what each destination received and how it acknowledged it, and save the report
  as CSV or JSON. Messages can also be exported with filters. Both are recorded in the audit log, and
  their files can contain patient data.

  Technical notes:

  `packie messages verify` produces per-destination transmission summaries and CSV/JSON reports with
  original/transformed hashes, ACK level/code and the ACK's own control id; `packie messages export`
  writes filtered CSV/JSON Lines with optional original bodies. Both are attributed and audited;
  outputs can contain PHI.

### Medium impact

- **Fixed:** Messages show the configuration version in effect after an in-place change (2026-11-17, [#5](https://github.com/openbga/interface-engine/pull/5))

  After a channel change that applied without a restart, new messages and the channel's status kept
  showing the version the channel started with. They now show the version in effect, matching their
  deliveries.

  Technical notes:

  New messages, the channel snapshot and bridge tap observations read the configuration version of
  the definition in effect; the snapshot's `config_version` follows in-place updates.

- **Added:** See what changed in What's new (2026-11-17)

  The What's new button in the console's top bar lists the changes to the engine and the console,
  newest first and most important first, and marks the ones you have not seen yet.

  Technical notes:

  The list is built from the entry files in `changelog.d/`, which also generate `CHANGELOG.md`
  (`npm run changelog` in `console/`). It ships with the console; nothing is fetched. Which changes
  you have seen is remembered by the console on this computer.

- **Fixed:** Backups, exports and reports are more dependable (2026-11-12)

  Fixed problems in backup restore, exports that continue over several runs, the order of the
  configuration history and the totals of transmission reports. File output keeps to its size limit,
  and endpoint tests no longer take a serial port another channel holds.

  Technical notes:

  Transactional backup restore, exclusion of secret values from backups, audited export
  continuations, configuration-history ordering and transmission report totals/truncation.
  Delimited-file output respects its configured size limit, and endpoint checks respect exclusive
  serial-port ownership.

- **Changed:** Bridge recordings are kept for 30 days by default (2026-11-12)

  Recorded bridge traffic is now removed after 30 days unless you set another limit, so recordings no
  longer grow without bound.

  Technical notes:

  Bridge recordings use `tap.retention`, defaulting to 30 days with no count limit. The retention
  run purges observations beyond either limit; `retention_unbounded` warns only when recording is
  enabled and neither limit is set.

- **Added:** Compare two configuration versions field by field (2026-11-12)

  See exactly what differs between two saved versions, the active configuration or a configuration
  file before you apply it.

  Technical notes:

  `packie config diff` compares saved versions, active configuration and runtime files field by
  field; `packie config export` aliases the active JSON export.

- **Added:** Find serial ports and test an endpoint before you rely on it (2026-11-12)

  The engine lists the serial ports this computer has, with their USB details and what already uses
  them. Test an endpoint's settings, saved or not, and see each step's result without sending any
  data.

  Technical notes:

  `packie endpoints list-serial` shows OS ports, USB details and configured use; `packie endpoints
  test` checks saved or unsaved serial, TCP, folder and SFTP settings without sending application
  data, with per-step results and `endpoint_test` audit records.

- **Added:** Guides for validation, go-live and daily operation (2026-11-12)

  The documentation now includes a validation record, a go-live checklist and procedures for audit
  review, backup and restore, change control and downtime, with guides for notifications, endpoint
  tests and transmission evidence.

  Technical notes:

  Laboratory safeguards crosswalk, validation record and go-live checklist; audit review,
  backup/restore, change-control and incident/downtime operating procedures; notification
  setup/recovery, endpoint discovery/tests and transmission/export evidence.

- **Changed:** Messages are stored and delivered several times faster (2026-11-12)

  The engine stores about 2,450 messages a second across 50 channels, up from 525, and each message is
  saved in well under a tenth of a second.

  Technical notes:

  Measured durable throughput improved from 525 to about 2,450 messages/s across 50 channels, with
  commit p99 reduced from 130–293 ms to 43–47 ms. One-channel throughput improved from 467 to about
  1,250 messages/s; idle CPU measured 1.8 % of one core.

- **Fixed:** More reliable recovery and delivery (2026-11-12)

  Fixed problems with shutting down during startup, back-to-back ASTM transfers, HL7 sessions that
  stop answering and recovery after a restart. Protocol settings and acknowledgement rules now hold
  through delivery and recovery.

  Technical notes:

  Early shutdown during startup, consecutive ASTM transfers, unresponsive HL7 sessions, retention
  races and retry of startup reconciliation. Protocol settings and acknowledgement policies remain
  consistent through delivery and recovery.

- **Changed:** Resending an older revision of a message is refused unless you confirm it (2026-11-12)

  When a newer revision of a message exists, resending the older one is refused by default, from the
  console and the command line. You can still override it explicitly, and the override is recorded in
  the audit log.

  Technical notes:

  Resending a superseded revision is refused by default in both modes. The explicit
  `--superseded-ok` override is audited and remains separate from duplicate-risk confirmation.

### Low impact

- **Fixed:** Help and guides corrected where they disagreed with the engine (2026-11-22)

  The Connection help article now gives the console's actual reconnection timing: a first retry
  after about half a second, doubling to about 5 seconds. The operator guides were corrected in
  the same way wherever they described something the engine does differently.

  Technical notes:

  Operator guide corrections: the macOS daemon's error log is
  `/Library/Logs/Positive ACK/launchd.stderr.log`; the Messages text filter searches identifiers
  and message type, never bodies; a truncated transmission CSV ends with a `# truncated:` line and
  its totals cover the whole period; an export with `--bodies` stops with an error when an
  original cannot be read; `max_frame_chars` accepts 64–64,000; an AU class A destination and an
  HL7 destination with `ack_mode: none` reach `handed_off` at most. No behavior changed.

- **Fixed:** Configuration fixes (2026-11-12)

  Signing off a verification no longer crashes, a configuration with extra text after it is rejected,
  and retry multipliers keep their exact value when saved.

  Technical notes:

  Verification sign-off no longer crashes; validation rejects trailing JSON text, and retry
  multipliers retain their precision when configuration is saved.

- **Added:** Long-running endurance and fuzz testing (2026-11-12)

  The engine was run for hours under crashes, full storage, destination outages and configuration
  changes without losing or duplicating a message, and its message readers are fuzz-tested every week.

  Technical notes:

  Chaos soak harness covering crashes, blocked storage, destination outages and configuration
  changes. A one-hour run handled 64,870 messages and an eight-hour run 565,791; after the two
  defects the long run exposed were fixed, a four-hour run handled 284,941. No run lost or
  duplicated a message. Twelve fuzz targets each ran clean for at least ten minutes after an
  allocation defect was fixed, with weekly fuzz runs configured in CI.

- **Changed:** Retention keeps an earlier revision a kept message still refers to (2026-11-12)

  Cleaning up old messages no longer removes an earlier revision while a newer, retained revision
  still refers to it.

  Technical notes:

  Retention keeps an earlier revision while a retained revision references it.

## 0.1.0 (2026-11-12)

### High impact

- **Added:** ASTM, AU, HL7 v2 and delimited files (2026-11-12)

  Channels can receive and send ASTM, AU400/AU640 and HL7 v2 messages, with acknowledgements, and read
  and write delimited files.

  Technical notes:

  ASTM framing and record codecs with receive and send drivers; AU400/AU640 fixed-width messages;
  HL7 v2 parsing, MLLP and original/enhanced acknowledgements; templated delimited-file reading and
  writing, with synthetic conformance fixtures.

- **Added:** Bridge and message channels over serial, TCP, folders and SFTP (2026-11-12)

  Bridge channels pass bytes straight between two endpoints, with a live transcript to watch the
  traffic. Message channels store each message before acknowledging it, filter and map it, and deliver
  it to one or more destinations with retries.

  Technical notes:

  Serial, virtual serial, TCP client/listener, folder and SFTP transports; bidirectional byte
  bridges with bounded buffers, pairing, reconnect, pause/resume and a live traffic transcript (one
  viewer, bounded, never stored); message channels with receipt-before-ACK, destination retries,
  duplicate detection, startup reconciliation, resend with a reason, declarative filters, mappings
  and code tables checked when a configuration is validated.

- **Added:** Command line for running and managing the engine (2026-11-12)

  The `packie` command starts the engine, shows its status, pauses and restarts channels, finds and
  resends messages, follows the logs, applies configuration, stores secrets and installs the service.
  Every command can also answer in JSON for scripts.

  Technical notes:

  `packie run`, `status`, `channels` (pause, resume, restart), `messages` (list, show, resend,
  close), `logs` (with `--follow`), `config` (show, validate, apply, history), `secrets` and
  `service` (install, uninstall, start, stop, status), with `--json` output and exit codes for
  scripts.

- **Added:** Desktop console for the engine (2026-11-12)

  The console shows the dashboard, channels, endpoints, messages, logs, alerts and settings, edits the
  configuration with version history, and has help on every page.

  Technical notes:

  Tauri 2 and React desktop application attached over local IPC; dashboard, channels, endpoints,
  messages, logs, alerts, settings, configuration editors, version history and page help; browser
  mock mode and generated TypeScript IPC types.

- **Added:** Every message is stored and can be searched (2026-11-12)

  The engine keeps each message as received and as delivered, with every delivery attempt, and you can
  search them. Old messages are removed on a schedule, never while work on them is unfinished.

  Technical notes:

  Bundled SQLite WAL store with one writer, original and transformed bodies, delivery attempts,
  message search, retention that preserves unresolved work, and migrations. If the store cannot be
  opened the engine starts degraded, keeps bridges running and retries.

- **Added:** Installers for Windows, macOS and Linux (2026-11-12)

  Each installer puts the engine in place and starts its service. The Windows and macOS installers
  include the console; the Linux package has the engine only.

  Technical notes:

  Windows NSIS installer, macOS package and Linux engine Debian package, each installing and
  starting the service; CI checks and a manually dispatched release-build workflow that includes a
  reproducible-build check of the engine on Linux and macOS.

  Known limitations: installers are unsigned and macOS packages are not notarized. Windows and Linux
  installer qualification remains pending; see [packaging/VERIFY.md](packaging/VERIFY.md). The Linux
  engine package does not include the console.

- **Added:** Management stays on the lab computer and secrets stay out of files (2026-11-12)

  Only accounts on the engine's own computer that are allowed to manage it can reach it. Passwords
  and other secrets are kept in the operating system's key store or a file only the service can
  read, never in logs, and listening endpoints accept only the peers you allow.

  Technical notes:

  Management interface limited to the local computer (socket group `packie`, pipe access control
  list); state and log directories private to the service account; secrets in the OS key store or an
  owner-only file and never in logs; listener peer allowlists.

- **Added:** The engine runs as a service on Windows, macOS and Linux (2026-11-12)

  The engine runs in the background as an operating-system service, or in a terminal, and the console
  and the command line manage it from the same computer. It keeps logs and raises alerts for
  endpoints, deliveries, storage, disk space and configuration.

  Technical notes:

  Rust engine that runs as an OS service (Windows service, launchd, systemd) or in a console, one
  instance per state directory. Local management over a Unix socket or Windows named pipe, validated
  and versioned configuration applied per channel, structured logs with a bounded in-memory ring,
  snapshots, and alerts for endpoint, delivery, storage, disk and configuration conditions.

### Medium impact

- **Added:** Operator, contributor and developer documentation (2026-11-12)

  Guides cover installing, configuring a first channel, running and monitoring the engine and
  troubleshooting, alongside the requirements and behavior specifications.

  Technical notes:

  Requirements, architecture, decisions, behavior specifications, acceptance criteria, simulator
  scenarios and benchmarks, operator guides, contributor and developer guides, security policy and
  installer verification checklist.

  Known limitation: serial scenario execution requires Linux pseudo-terminals and is skipped on
  macOS.
