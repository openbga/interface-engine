# PDF report specification

How a message channel reads a laboratory report published as a PDF file (protocol
`pdf_report`): text extraction, the template that locates labelled fields on the page, the
model that filters and mappings address (including the file itself, for embedding), failures,
limits and the test tool. Connector settings sit in
[configuration.md §7.7](configuration.md#77-pdf_report); receipt, states and duplicates follow
[message-channels.md](message-channels.md). Requirements: PRO-06, PRO-07, EPT-03, EPT-04.

## 1. Scope

| Rule | Statement |
| --- | --- |
| Source only | `pdf_report` is a source protocol of message channels on `folder` and `sftp` endpoints. It is never a destination protocol; its model cannot be written. |
| One file, one report | A complete file is one message. Its original bytes are the file's bytes, unchanged. |
| Text layer only | Text is read from the page content of a born-digital PDF. Scanned images are not recognized (no OCR); a report without a text layer is `invalid` (`pdf_no_text`). |
| Labelled fields | The template names the labels whose values the engine reads. A declared label that the report does not carry gives an empty value (unless the field is `required`), so a report with fewer data than its layout allows is still read. Result tables, numbered comments and other unlabelled text are not read; the whole report travels as the embedded file (section 5). |
| Declared layout | Nothing is inferred from a report that the template does not declare. An unknown label in the label area stops the report (section 4.1), so a renamed or added label is never read as an empty or a neighbor's value. |
| No active content | Scripts, forms, annotations, attachments, links and external resources are ignored and never fetched or executed. |

## 2. Receipt

| Step | Behavior |
| --- | --- |
| Read | The endpoint finds a complete file (configuration §5.4: pattern, stable size, ignored suffixes). A file over `max_file_bytes` is not read; one `aborted` message (`file_too_large`) records it, as for delimited files. |
| Commit | The whole file is committed as one `received` message: original bytes, `source_file` (relative path, SHA-256 of the file, no row range), duplicate check, and the identifiers of section 2's table, read from the report just before (a report that does not parse is committed without them). |
| Parse | The pipeline's parse stage ([message-channels.md §6](message-channels.md#6-pipeline)) extracts the text (section 3) and applies the template in force (section 4). Failure: the message becomes `invalid` with a reason code and `reason_detail` (section 7); no delivery is created. |
| Consume | The source file is moved, deleted or left per the endpoint's `on_consumed`. A moved file goes into `<directory>/YYYY/MM/DD` (the engine host's local calendar date at `received_at`, zero-padded, created when needed). Without `filtered_directory` and `invalid_directory`, `<directory>` is `processed_directory` right after the commit. With either set, the move waits until parsing and the source filters have decided the message: `routed` to `processed_directory`, `filtered` to `filtered_directory`, `invalid` or `aborted` to `invalid_directory` (`null` uses `processed_directory`; [message-channels.md §2.2](message-channels.md#22-per-protocol)). |
| Isolation | Extraction runs off the channel's I/O tasks within the limits of section 8, so a slow or hostile file never stalls receipt on this or another channel. |

Because the original is stored before parsing, a report that failed because the template was
wrong is recovered by correcting the template and resending the message with `new_message`,
which parses the stored original with the active configuration.

| Item | Value |
| --- | --- |
| `message_type` | `pdf_report` |
| `control_id`, `sample_id`, `patient_id` | The fields of those names (section 4.2), else `null`. |
| Source identity (duplicates) | Relative file path, as for delimited files. |
| Content hash | SHA-256 of the file's bytes. A file uploaded again unchanged under the same name is a `duplicate`; the same name with other content is processed with `revision_of` set. |

## 3. Text extraction

| Rule | Statement |
| --- | --- |
| Content | Text drawn by each page's content streams, including the form XObjects they draw, with the graphics state (`cm`, `q`/`Q`) and the text state applied. |
| Coordinates | PDF points (1/72 inch), origin at the top left of the page as displayed: the crop box with the page rotation applied. |
| Fonts | Simple fonts use one byte per code; composite (`Type0`) fonts are read with the `Identity-H` encoding (two bytes per code), and another encoding makes the file `pdf_unreadable`. Each code's text comes from the font's ToUnicode map, else (simple fonts) from its encoding. Advances come from the font's widths, else for the 14 standard fonts from their metrics, else from the descriptor's missing width or half an em. |
| Direction | Only glyphs that run left to right on the displayed page are read; text drawn at an angle or vertically is not. |
| Words | Glyphs on one line form a word until a space character or a horizontal gap wider than 0.25 × the font size. A glyph drawn again at almost the same place (simulated bold) counts once. A word has its text, page, bounding box and font size (the effective size after text and page scaling). |
| Unmapped glyphs | A glyph that is read (left to right, see Direction) and whose character cannot be determined makes the report `invalid` (`pdf_no_text`), because a value read without it could be wrong. |
| Text | Text is NFC-normalized; no-break spaces count as spaces; control characters are dropped. |
| Lines | Words whose vertical centers differ by at most half of the smaller font size are on one line; a line's words are ordered left to right. Lines are ordered top to bottom per page. |
| Phrases | On a line, consecutive words separated by a gap smaller than `phrase_gap` × the font size of the left word form one phrase; a wider gap starts a new phrase. A phrase's text is its words joined by single spaces. |
| Matching | Template texts (labels, `label_area_end`) match words of the page compared without case and with runs of whitespace collapsed; a template text matches whole words only. |

A page with no words is allowed (a blank page); a report with no words on any page is
`invalid` (`pdf_no_text`). A file whose page tree yields no page, or fewer pages than it
declares, is `pdf_unreadable`: a damaged file is never read as a shorter report.

## 4. Template

The template is the connector's `settings` (configuration §7.7): the fields to read and where
the label area ends.

### 4.1 Labels

Every label of every field is located on every page before values are read. Where labels
overlap, the longest label wins and the words it covers are not matched again (so a field
labelled `DOB:` does not match inside `Mother DOB:` when that label is also declared).

The **label area** runs from the line of the first label found on page 1 down to the first
line whose first phrase begins with a `label_area_end` text, or to the end of page 1. In the
label area, a word ending with `:` that is not part of a found declared label is an **unknown
label**: the report is `invalid` (`template_mismatch`). Labels outside the
label area (letterhead, results, footers) are not checked. There a value stops only at a
declared label or a phrase gap, so declare the labels whose values are needed and the labels
that follow them on the same line (a footer written as one run of text,
`Last Name: … Accession Number: …`).

### 4.2 Fields

| Field setting | Meaning |
| --- | --- |
| `name` | The model name: `field.<name>`. |
| `label` | One label, or 1–8 alternative labels; the first that is found on the page is used. |
| `position` | `right` (default): the value is on the label's line. `below`: the value is in the block of lines under the label. |
| `occurrence` | Which match of the label on a page: 0 the first in reading order (default), 1 the second, −1 the last. |
| `first_page_only` | `true`: the field is read on page 1 only and its label on later pages is ignored, for a label that a footer repeats with a different value (a grid's `DOB: 1/2/2026 08:05` and a footer's `DOB: 1/2/2026`). Default `false`. |
| `line`, `phrase`, `indent` | `below` only, see below. |
| `required` | `true`: the label is found on page 1 and its value is not empty; otherwise `template_mismatch`. `false` (default): a missing label or value is an empty field. |
| `type`, `format` | As delimited columns: `string` (default), `integer`, `decimal`, `date`, `datetime`; `format` is required for dates and may be an array of 1–4 formats, the first that matches the whole value wins. A non-empty value that matches no format, or is not an integer or decimal, is `field_invalid`. Values stay text in the model. |

**`right`.** The value is the rest of the label's phrase after the label, or, when the label
ends its phrase, the next phrase on the line. It stops before the next label on the line. When
the next phrase begins with a label, the value is empty.

**`below`.** The value is read from a block of lines under the label on the same page. Only
phrases whose left edge is at least the label's left edge minus 4 pt count; phrases further
left belong to another column and are ignored. A line belongs to the block when its first
counted phrase starts between the label's left edge − 4 pt and the label's left edge +
`indent` (default 4 pt). The first block line is within 3 × the label's font size below the
label; each next one within 2 × the font size below the previous one. Lines in between that do
not belong (another column's text) are skipped. The block ends at a larger gap, at a line
whose first counted phrase begins with a label, or at the end of the page. The value is
phrase `phrase` (0-based, default 0; −1 the last counted phrase) of block line `line` (1-based,
default 1); a missing line or phrase is an empty value. The last phrase finds a value printed at
the right of a row whatever precedes it, such as a facility code after a name the report may
leave blank (give `indent` enough room for a row that starts at the code).

**Pages.** A field is evaluated on every page where its label occurs (page 1 only with
`first_page_only`), with `occurrence` counted on that page. Every page that yields a non-empty value must yield the same value;
otherwise the report is `invalid` (`template_mismatch`, the field and pages named). A file
that holds several reports is refused this way, because their patient fields differ.

## 5. Model and paths

| Path | Value |
| --- | --- |
| `field.<name>` | The field's value; empty when the label or its value is absent. |
| `report.pages` | The number of pages. |
| `report.file_base64` | The original file, Base64-encoded (standard alphabet with padding, no line breaks), for embedding the report in a mapped message, for example an HL7 `ED` value. It holds none of the HL7 or ASTM delimiters. It is about 4/3 of the file's size, so the destination's `max_message_bytes` must allow for it; a larger mapped message fails the delivery with `transform_error`. |

Paths are checked against the template when the configuration is validated: field names must
exist. The model is read-only: no `to` path or `into` record can be a PDF path.

## 6. Viewing and export

The views read a stored report again with the template of its channel in the active
configuration (its phrase gap and page limit; when the channel no longer reads PDF reports,
the text views use the defaults).

| Surface | Behavior |
| --- | --- |
| Message detail, original body | The text view is the extracted text: a `Page n` line before each page, one line per text line, phrases separated by a tab shown as `<HT>`. A report whose text cannot be extracted answers `unavailable` with the reason; the hex view shows the file's bytes. Audited as any body view. |
| Parsed fields | `get_message_fields` lists the model's non-empty `field.` values in template order, then `report.pages`; `report.file_base64` is never listed. When the channel no longer reads PDF reports, or the report does not read with the template, the reason is given instead, in template terms. |
| Export with bodies | The body column holds the extracted text, with tab characters between phrases; it is empty for a report whose text cannot be extracted (its state and reason say why). |
| Logs, alerts, snapshots, `reason_detail` | Never the extracted text or a value. |

## 7. Failures

| Reason code | When |
| --- | --- |
| `pdf_unreadable` | The file is not a PDF, is damaged beyond its cross-reference recovery, needs a password to open (a file restricted only by permissions opens), or uses a feature the reader does not support (a composite font encoding other than `Identity-H`, a damaged content stream, form XObjects nested more than 16 deep or in a cycle, graphics states saved more than 256 deep). |
| `pdf_no_text` | No page has text, or a glyph's character cannot be determined. |
| `pdf_limits` | A limit of section 8 was reached. |
| `template_mismatch` | A required label or value is missing, an unknown label in the label area, or conflicting field values between pages. |
| `field_invalid` | A typed field does not match its type or formats. |

`reason_detail` (message record field, at most 300 characters) says what failed in terms of the
template and position only, for example `label "Lab Number:" not found on page 1`,
`unknown label in the label area on page 1, line 9`, `field "dob" does not match its formats`.
It never contains text read from the report. It is shown in the message detail and logged with
`message.invalid`.

## 8. Limits

| Limit | Value |
| --- | --- |
| File size | `max_file_bytes` (default 20 MiB). |
| Pages | `max_pages` (default 20); more is `pdf_limits`. |
| Decompressed content | 64 MiB per file across all streams; more is `pdf_limits`. |
| Words | 200,000 per file; more is `pdf_limits`. |
| Time | 10 s of extraction per file; more is `pdf_limits`. |
| Panics | Malformed input yields a reason code; parsing never panics (PRO-06). |

## 9. Test tool

`test_pdf_report` (IPC; console **Test with a PDF…** in the source connector editor and
**Test against the current template** on an `invalid` `pdf_report` message; CLI
`packie pdf test`) runs sections 3–5 without storing anything.

| Input | Meaning |
| --- | --- |
| `settings` | A `pdf_report` template, written as in the document; checked by the connector rules first (problems as `invalid_config`, with paths relative to the connector such as `settings.fields[2].label`, or under `destination.`). |
| `file` or `message_id` | A PDF (Base64, at most 8 MiB decoded), or a stored `pdf_report` message whose original is used; exactly one. |
| `destination` (optional) | `protocol`, `settings`, `mapping` of a destination, written as in the document; the report is parsed, mapped and encoded as a delivery would be, with a new control id and the current time. |

The result has `ok`, the reason code and detail on failure, the lines with their phrases and
positions (to write or correct a template), the model without `report.file_base64`, and the
encoded destination message or its transform error; in the encoded message a value longer
than 1,000 characters (the embedded file) is shortened to its first 100 characters and its
length. It shows report content, so it is audited (`pdf_report_test`, with the message id when
one is used) and the console draws it only in the test's dialog, which drops it when it closes or
the console goes idle.

## 10. Shipped profile

The profile catalog (configuration §7.6) carries a template and a mapping for one newborn
screening report layout, built from one synthetic report; validate them with the laboratory's
own reports before clinical use.

| Profile | Content |
| --- | --- |
| `pdf_newborn_screening_report` (connector, source) | Every label of the patient, specimen and collection grid as a `right` field, none required (`patient_name`, `gender`, `mother_name`, `dob`, `birth_order`, `mother_dob`, `race`, `birth_weight`, `mother_phone`, `medical_record`, `gestational_age`, `meconium_ileus`, `kit_number`, `nicu`, `collected`, `sample_id` = `Lab Number:`, `age_at_collection`, `received`, `specimen_type`, `transfusion_date`, `reported`, `food_source`, `collection_weight`, `printed`); `dob` and `collected` are dates printed with or without a time (`M/d/yyyy HH:mm` or `M/d/yyyy`), and `dob` is read on page 1 only (`first_page_only`), because the footer repeats `DOB:` without the time on every page; the footer's `last_name` (`occurrence` −1), `accession_number` and `Reported Date:`; the report recipient's first line as a `below` field (`recipient`, `indent` 60; the rows after it vary between reports and are not read); the birth hospital, collection facility and physician or clinic (`Physician/Clinic`, printed without a colon) as `below` fields, each with its facility code as the last phrase of the row (`phrase` −1, `indent` 170: `birth_hospital_code`, `collection_facility_code`, `physician_facility_code`), which is found also when the report leaves the facility's name blank. `label_area_end`: `Results`. |
| `pdf_newborn_screening_to_hl7_oru` (mapping) | `ORU^R01^ORU_R01` for an `hl7_file` destination; the Lab Number identifies the patient and the order. `PID-2`, `PID-3` (`<Lab Number>^^^<assigning authority>^PI`), `PID-18` = `sample_id`; `PID-5` family name `last_name`, given names `patient_name` without it (`strip_suffix`); `PID-7` `dob` (to the minute when the report prints a time, otherwise the date; likewise `OBR-7` and `SPM-17`); `PID-8` through the `sex` table; `PID-10` through the `race` table; `PID-16` `S`. `NK1` when the report names the mother: `NK1-2` `mother_name`, `NK1-3` `MTH`, `NK1-5` `mother_phone`, `NK1-7` `N`. `PV1-2` `N`; `PV1-3.4` `birth_hospital_code`; `PV1-4` `N`; `PV1-7` and `PV1-8` the report recipient (the ordering provider); `PV1-19` `sample_id`; `PV1-44` `received`. `ORC-1` `RE`, `ORC-3` `sample_id`, `ORC-21` physician or clinic with its facility code, `ORC-23` the laboratory's phone. `OBR-3` `sample_id`; `OBR-4` the panel from `specimen_type`; `OBR-7` `collected`; `OBR-14` `received`; `OBR-16` the report recipient (name without NPI); `OBR-17` the laboratory's phone; `OBR-22` `reported`; `OBR-25` `F`. One `OBX` of type `ED`: `OBX-3` the panel, `OBX-5` `^AP^PDF^Base64^` with `report.file_base64`, `OBX-11` `F`, `OBX-14` `reported`. `SPM-2` `kit_number`, `SPM-17` `collected`, `SPM-18` `received`. The laboratory's phone, the assigning authority and the panel's coding system are constants a deployment fills in; MSH-3 to MSH-6 come from the destination settings. A field the report does not carry leaves its HL7 field empty: no rule is `required` and none writes the HL7 null `""`, which a receiver would read as "delete the stored value". |

A channel that converts only some hospitals' reports adds a source filter on the birth
hospital's facility code, for example
`{"path": "field.birth_hospital_code", "op": "in", "value": ["F0001", "F0002"]}`; the other
reports are stored as `filtered` and, with `filtered_directory`, moved apart. Removing the
filter sends every hospital's reports.
