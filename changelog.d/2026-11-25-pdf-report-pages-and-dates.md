---
date: 2026-11-25
type: fixed
impact: medium
title: Multi-page newborn screening reports and dates without a time are read
area: Channels
---
The newborn screening report preset now reads reports that run to two or more pages, where
the footer repeats the date of birth without its time, and reports that print the date of
birth or the collection date without a time. Such reports were refused as not matching the
template; now the HL7 message carries the date alone when the report prints no time.

## Technical notes

New PDF template field setting `first_page_only` (default `false`): the field is read on
page 1 only. Profile `pdf_newborn_screening_report`: `dob` has `first_page_only: true`;
`dob` and `collected` are `type: date` with formats `M/d/yyyy HH:mm` and `M/d/yyyy`.
Mapping `pdf_newborn_screening_to_hl7_oru`: PID-7, OBR-7 and SPM-17 are written as
`yyyyMMddHHmm` when the value has a time, otherwise `yyyyMMdd`. Channels created from the
earlier presets keep their settings; apply the presets again or make the same edits.
