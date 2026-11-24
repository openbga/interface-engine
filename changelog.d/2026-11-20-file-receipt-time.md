---
date: 2026-11-20
type: fixed
impact: medium
title: Files are received when the engine takes them
area: Messages
link: /messages
---
A message read from a file (a delimited file or a PDF report) was recorded as received at the
file's last-modified time. An older file dropped into a folder today appeared days back on the
Messages page, and retention could remove it almost at once. Such messages are now received at
the moment the engine takes the file. Files are still read oldest first.
