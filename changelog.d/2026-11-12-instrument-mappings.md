---
date: 2026-11-12
type: added
impact: high
title: Calculations, presets and more protocol settings for instrument channels
area: Channels
link: /channels
---
Mappings can now calculate exact values, build text, add records and check preconditions, and
ready-made presets fill in the settings and mappings for common instrument layouts. The channel
editor shows each preset's values and a preview before you save.

## Technical notes

Exact-decimal calculations, text steps, appended records, `require` preconditions and `for_each`
modes in mappings; six connector presets and seven mapping presets for common instrument layouts,
with console editors and previews; ASTM `enq_attempts`, `busy_retry_ms`, `sample_id_path` and
frames up to 64,000 characters; AU `test_code_digits`; HL7 `require_end_cr`, `ack_message_types`
and `reply_type` for order responses. Specimen identifiers (ASTM O-4, HL7 SPM) reach receipt
metadata and search.
