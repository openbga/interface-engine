---
date: 2026-11-12
type: changed
impact: high
title: On Windows, members of PACK-IE Operators can manage the engine
area: Engine
---
Besides administrators, members of the local PACK-IE Operators group can now use the console and the
command line on Windows. Installing the service creates the group and adds the installing account;
add other operators to it.

## Technical notes

Windows management access now uses the local `PACK-IE Operators` group besides SYSTEM and
Administrators. Default `ipc.group: "packie"` selects this group; service installation creates it
and adds the installing account. Unix continues to use the `packie` group by default.
