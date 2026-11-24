---
date: 2026-11-19
type: added
impact: high
title: Read PDF reports and send them on as HL7 messages
area: Channels
link: /channels
---
A message channel can now read laboratory reports published as PDF files from a folder or SFTP
server. A template names the labels printed on the report; their values fill the outgoing HL7
message, and the report itself can travel inside it. Reports a filter leaves out and reports
that cannot be read can be moved to their own folders, set in the folder or SFTP endpoint editor
under **After a file is read**. **Test with a PDF** in the channel
editor shows what a template reads from a report and the message it would send, before you
apply anything. A stored report shows as the text read from it, and its fields as the
template reads them. A newborn screening template and mapping are included as presets.

## Technical notes

New source protocol `pdf_report` (text layer only, no OCR) with the template settings of
configuration §7.7 (a `below` field's `phrase` −1 is the last phrase of its line); mappings read `field.<name>`, `report.pages` and `report.file_base64`.
Folder and SFTP endpoints gain `filtered_directory` and `invalid_directory`. New text step
`strip_suffix`; date formats accept the unpadded tokens `M`, `d` and `H`. Invalid PDF reports
record a `reason_detail`; the message store migrates to schema version 11 when it opens. The
test tool is the `test_pdf_report` management request and `packie pdf test`, audited as
`pdf_report_test`. New dependency: `lopdf` (MIT).
