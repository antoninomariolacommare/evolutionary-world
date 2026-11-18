# Schema-8 checkpoint fixture

`checkpoint-v8.json.gz` is a complete checkpoint emitted and accepted by the Rust
engine at commit `7d49cd6d22011ad08ab784b5c8870c882dc09b6d`. It is test data, not
an example of an evolved world.

Construction (without changing the engine):

1. Run `headless --seed 0 --ticks 0 --save seed-zero-v8.json`.
2. In a copy, replace the terrain tiles with 320,000 zeroes, and every numeric
   grid layer with a single `rle1:` run of 320,000 zero values (little-endian f32
   for float layers; u8 for biome and road). Clear volcano/pool lists. Retain
   all 120 organisms, 14 animals and the complete schema/runtime sections.
3. Set RNG stream to `18446744073709551615` and word position to
   `36893488147419103239` (2^65 + 7). Both are valid ChaCha values which would
   lose precision if JavaScript parsed and reserialized the save.
4. Run `headless --load fixture-input.json --ticks 0 --save checkpoint-v8.json`.
   The strict Rust loader accepts it and its writer produces the canonical file.
5. Gzip with mtime 0 (Python `gzip.compress(bytes, mtime=0)`).

Uncompressed size: 1,769,184 bytes. Compressed size: 251,424 bytes.
Uncompressed SHA-256:
`71ce3b328025546b1fd148e45e7c09b301b7444c9d8a822d2ad1dbe52a19749f`.

The desktop tests decompress this file without changing it, and assert exact
byte preservation through validators, real worker threads, exports and folder
copies. Small legacy test payloads separately exercise the desktop's legacy
shape/seed policy; they are not substitutes for a full Rust validation fixture.

Desktop validation is a transport/metadata gate. Rust remains responsible for
complete authoritative-state validation and migration. The IPC tests mock
Electron dialogs, settings and the running simulation/checkpoint endpoint; they
use the actual desktop handlers, locks, file transactions and worker threads.
They do not constitute a packaged Windows/MSI or graphical end-to-end test.
