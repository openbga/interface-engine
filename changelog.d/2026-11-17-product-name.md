---
date: 2026-11-17
type: changed
impact: high
title: The product is now called OpenBGA Interface Engine
pr: 3
---
The engine and console carry the new name. Alert e-mail subjects, export files and installers use it
too, so update any mail rules or scripts that look for the old names. The command, the service and
the install folders keep their names.

## Technical notes

The product is OpenBGA Interface Engine (repository `openbga/interface-engine`, console OpenBGA
Interface Engine Console). Alert e-mail subjects start `[Interface Engine <node>]`, exports are
named `interface-engine-*`, installers `openbga-interface-engine-<version>-*`. Install folders,
the `packie` command, service, group, variables and the Windows group `PACK-IE Operators` keep
their names (see [install](docs/operations/install.md)).
