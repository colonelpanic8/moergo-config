# Magic runtime crash candidate

This branch is an offline candidate, not a hardware-qualified installation.
The default installation remains rolled back. No keyboard was accessed during
this work; testing either half requires separate authorization and recovery
artifacts. Local evidence and build outputs are under `.handoff/` and are not
part of these commits.

The runtime apply evidence stops during keymap writes, before lighting writes.
It does not identify the faulting instruction or prove stack overflow. A host
replay through the actual Rynk service and storage task reproduces uncached
garbage collection doing 339,243 ready flash reads in one executor poll.
The storage adapter now yields after at most 32 flash operations. Repeated
bulk writes force collection and preserve serialized key actions, including
tap-hold profile indices, after reopening the same flash bytes.

Shared lighting engines now live in static storage and the processor borrows
them. An explicit command call boundary limits temporary stack growth. Both
boards retain their previous table capacities and protocol/storage layouts.
Full-capacity host tests cover atomic conditional-table replacement, advanced
layer/lock predicates, brightness, output controls, and 20-second Magic linger.

A second offline investigation reproduced a separate configuration-read race.
Two real Rynk sessions requesting different persisted layer names received each
other's replies; cancelling a queued reader made the next session consume its
stale reply. USB and BLE sessions share the service and can run concurrently.
Each storage read response now serializes its callers and drains an outstanding
reply after cancellation before queuing another request. This applies to layer
metadata, BLE names, bonds, peer addresses, connection type and active profile.
It preserves all request messages, wire formats and storage records. The same
session/storage scenarios now return the correct names. This race is proven,
but does not explain the historical USB reset by itself.

The candidate carries all 36 previous RMK assembly pins plus
`fix/cooperative-storage` and `fix/storage-read-replies`. Its generated tree is
`66381c90e1ce6a0adfd5c5298abcbaeb44d227e2`, reproduced by a locked build.
The assembled candidate is published to `candidate/magic-runtime-crash`;
the assembly and product sources use `fix/magic-runtime-crash`. No default
published installation branch was moved.

The ordinary build-hash mismatch policy remains unchanged: a different
firmware build can reinitialize persisted configuration. Storage-format
compatibility does not imply that an upgrade automatically preserves settings.
Any later hardware qualification must account for the saved runtime backup.

## Final candidate inputs and artifacts

Both bundles were built with Rust 1.97.0 from clean product revision
`ca18b2a9b164fb6aca7c745eff1531898a59c0c5` and clean configuration revision
`8393967bff84eb8bb74f5307f5d0e12f5d555ac2`. The latter is the source/config
commit immediately before this results-only documentation update. The product
pins RMK `18eb611ef3e5fc1b13e06367251c9d16c9467d66` and assembly
`e73f9cb3f2769d4e3b27844a4b6f1fa3a8d8e007`; the standalone storage topic is
`3abef32c5bfab2a07df8850a71971944c7f31a97`. The read-response topic is
`5e3eaeb9fbcf` (full pin in the assembly lock).

Bundles, manifests, SHA256SUMS and DO-NOT-FLASH markers are in
`.handoff/candidate/glove80/` and `.handoff/candidate/go60/`. Each contains both
ELFs and UF2s. Every file hash and every UF2 block's family, address, sequence
and size was verified. Both manifests report `dirty: false` for source and
configuration. Application addresses start at `0x26000`; ends below are exclusive.

| Image | Candidate end | Prior failing candidate end | Change | Family |
| --- | --- | --- | --- | --- |
| Glove80 left | `0xd8200` | `0xd7d00` | +1,280 bytes | `0x9807b007` |
| Glove80 right | `0x8b300` | `0x8b300` | 0 bytes | `0x9808b007` |
| Go60 left | `0xd6d00` | `0xd6e00` | -256 bytes | `0x9809b007` |
| Go60 right | `0x8a300` | `0x8a200` | +256 bytes | `0x980ab007` |

The saved recovery Glove80 ranges end at `0xd5800`/`0x89f00`. This candidate
therefore uses 10,752/5,120 more flash bytes than that older recovery firmware,
while fitting the existing `0xdc000` application boundary. Recovery UF2s were
reconstructed offline from the supplied ELFs into `.handoff/candidate/recovery/`;
their hashes exactly match the original recovery manifest. The supplied
evidence files remain intact.

## Stack evidence

These are direct entry-prologue allocations from the supplied failing ELF and
final candidate ELF, excluding saved registers and nested calls. They are not
worst-case stack bounds and do not establish the historical reset's cause.

| Glove80 central measurement | Failing candidate | Fixed candidate |
| --- | ---: | ---: |
| Main poll local frame | 19,788 | 19,796 |
| `central_lighting::init` local frame | 36,580 | 396 |
| Lighting command local frame | 41,652 | 27,716 |
| Main task static pool | 117,384 | 100,000 |
| Separate static engine | 0 | 18,120 |
| Available linker stack region | 96,336 | 95,472 |

The engine wrapper's constructor frame is 18,128 bytes and its separate
`make_engine` frame is 22,452 bytes. Moving the engine eliminates large
by-value transfers; it does not eliminate all constructor temporaries.
Total static RAM increases by 864 bytes relative to the failing candidate
(including response serialization). Final Go60 linker stack regions are
94,376 bytes central and 161,176 peripheral; its central lighting-command
frame is 25,388 bytes. Disassemblies, symbol sizes and prologue reports for
all four final ELFs are under `.handoff/candidate/analysis/`.

## Execution of the shipping ARM instructions

Unicorn 2.1.4 executes each final ELF's actual lighting constructor and command
handler. DWARF supplies its optimized Rust layouts; the test loads the ELF's
ROM/RAM segments, initializes the static engine, stages the full 100/80-cell
advanced conditional table, verifies atomic visibility and exact paged readback,
then exports/applies a replica and verifies all cells again. All four images pass.

The emulator records the minimum stack pointer at every executed instruction,
including nested callees and saved registers. These are measured paths, not
whole-firmware upper bounds:

| Image | Constructor peak | Largest exercised command peak | Linker stack region |
| --- | ---: | ---: | ---: |
| Glove80 left | 45,388 | 32,500 | 95,472 |
| Glove80 right | 43,972 | 35,572 | 165,136 |
| Go60 left | 39,972 | 29,452 | 94,376 |
| Go60 right | 38,660 | 32,460 | 161,176 |

The harness calls these functions directly: the calling async task frames and
interrupt stack use are excluded. It models SCB thread mode for the mutex;
it does not model hardware transports, flash timing, interrupts or watchdog
expiry. Instruction counts are not CPU-cycle timing. No device was accessed.
Scripts/logs are under `.handoff/stress/`; reports and ELFs are under
`.handoff/candidate/`. The superseded first offline candidate is preserved in
`.handoff/candidate-v1/`.

## Verification and limits

- Real Rynk sessions replay 84 fragmented bulk requests, enqueue 2,352 key
  updates, force actual sequential-storage garbage collection, and reopen the
  same NOR-flash bytes. The original code fails at 339,243 ready reads per
  poll; the fix stays at 32. The assembled replay also passes with the actual
  Glove80 compile-time TOML, with 17 erases and exact serialized action readback.
- All 19 focused storage tests pass on the final source with the actual
  Glove80 compile-time TOML. These include four response concurrency/cancellation
  cases, two metadata-helper integration cases, the bulk-write replay and storage
  migrations. The standalone read-response topic passes strict Clippy and ARM
  no-std compilation; the assembled non-BLE Rynk/lighting/storage build passes.
  An independent audit confirmed cancellation safety at every await boundary.
- Real public Rynk session + NOR storage tests reproduce crossed and stale
  replies before the fix, and correct distinct names afterwards. Evidence is in
  `.handoff/candidate/reply-signal-audit/` and `.handoff/stress/reply-*-fixed.log`.
- The earlier 132 selected RMK lighting/storage/Control-GUI tests pass across generic
  and board settings. One pre-existing scene test assumes profile index 3:
  generic three-profile settings reject it, while the board's four-profile
  settings pass. The storage replay also passed under both settings.
- 117 RMK type/protocol tests including snapshots; 39 native Rynk library tests
  and its doctest; 15 shared lighting/replication host tests; 121 product host
  tests (two reporting-only tests intentionally ignored).
- Both runtime TOMLs and both evidence runtime fixtures validate using physical
  keys, emitters and zones taken from the corresponding source board TOMLs.
  Both stock-default parity checks pass, retaining only the allowed Glove80
  bilateral-thumb difference. No runtime configuration was applied.
- Both board `check --bins` checks, the project's `xtask check` host/WASM
  checks, the Rynk WASM build, RMK-types ARM no-std check and standalone storage
  topic ARM no-std check pass. Changed Rust files pass formatting. The storage
  topic passes Clippy with warnings denied.
- Assembled Clippy retains existing warnings; its pre-existing constant
  comparison lint was allowed. Shared host-test Clippy likewise allows the
  existing constant-assertion lint. WASM emits existing tsify deprecation
  warnings. An additional optional Rynk **no-alloc host-client** build still
  fails on pre-existing `alloc`/Extended API imports; those files are unchanged
  by this fix. The native client, WASM client and embedded firmware builds pass.

Logs and offline replay/build/inspection helpers are retained in
`.handoff/candidate/` and `.handoff/stress/`. The final source reran the 117
protocol tests, 39 native client tests and doctest, product host/WASM checks,
formatting, assembled Clippy, both firmware builds and all four ARM emulations.
No full RMK suite or known-hanging Rynk loopback test was run. The scheduling and read-response defects are reproduced and fixed; hardware
watchdog timing, whole-firmware stack high-water marks, and the original reset
mechanism remain unproven. The separate pre-existing shared write-completion
signal can acknowledge an unrelated queued write; this investigation did not
change that API or establish it as a reset cause.

Future hardware qualification requires separately authorized test hardware:
qualify the central with the saved recovery available, repeat runtime applies
through collection and reconnection, verify persistence, then qualify the
matching peripheral and Magic/brightness/output behavior on both boards.
Do not move the default installation pin based solely on these offline results.
