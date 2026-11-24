---
date: 2026-11-24
type: fixed
impact: medium
title: SFTP endpoints connect to servers with RSA host keys
area: Endpoints
link: /endpoints
---
SFTP endpoints now connect to servers that identify themselves with an RSA host key, including
older 1024-bit keys, and sign in with RSA private keys. Before, such a server failed at connect
with "Wrong server signature". Pin the server's `SHA256:` fingerprint as for any other key.

## Technical notes

The engine's SSH library is built with RSA support: `russh` 0.65.1 → 0.65.4 with its `rsa`
feature, which adds `rsa` 0.10.0, `pkcs1` 0.8.0 and `crypto-primes` 0.7.2 (MIT or Apache-2.0)
and takes `ssh-key` 0.7.0-rc.12. `rsa` 0.10.0 is the release that fixes RUSTSEC-2023-0071.
