# Refreshed RMK qualification — jay-lenovo, 2026-09-07

**Result: blocked before flashing; refreshed firmware is not hardware-qualified.**
The Glove80 firmware rebuild succeeded, but the pinned host control tool fails
to compile. Neither half was flashed, no bootloader request was sent, and no
runtime configuration was changed. Existing firmware remains installed.

## Build and artifact evidence

Read `HANDOFF-refreshed-rmk.md`, then `AGENTS.md`, before device interaction.
The machine reports `jay-lenovo`. The checkout was detached at the requested
configuration commit, with these pins:

| Component | Commit |
| --- | --- |
| Configuration | `3b605fd4d3f2e293bde56a1dc2922d9f77e97e50` |
| moergo-rmk | `02b82f1e36581d177a9e55a31adbfa1fa4ce5d1f` |
| RMK assembled | `ef9787302c52fe8a550f4d41b788602e63b73ad8` |
| RMK assembly | `0e3d2143866aa9d828fef1a14c62558458ab65c5` |

Ran `just firmware` successfully on this checkout. Both stock-configuration
parity checks passed as part of that command. It used the outer
`config/firmware.toml` and recorded the outer repository provenance. The product
and RMK working trees were clean. The outer checkout already contained the
untracked handoff and `dependencies/glove80-rmk/`; both were preserved. Its
manifest consequently reports `configuration.dirty = true`.

The rebuilt UF2s were byte-identical to copies saved from the already successful
local build. Independently parsed every UF2 block and checked magic values,
sequence/count, contiguous addresses, payload size, family ID, and manifest
SHA-256. Address ends below are exclusive.

| Half | Address range | Family | Blocks |
| --- | --- | --- | --- |
| Left/central | `0x26000–0xd9300` | `0x9807b007` | 2867 |
| Right/peripheral | `0x26000–0x8a200` | `0x9808b007` | 1602 |

These ranges and family IDs match the handoff exactly. Local SHA-256 values:

```text
left  8973c7d74fcb24a400ff878dfbaa7b704a6c8d7f2e3fefa73ca654387c2ca32b
right 4c529ed0d543279578ea9a2bd0870e0370a38125e550fcda1e7387f57106952d
```

The handoff's cross-machine hashes differ. Local repeatability is established;
cross-machine byte reproducibility and the cause of that difference are not.
No RAM or stack measurement was made.

## Blocking host-tool defect

This command failed twice with exit status 101 before opening the keyboard:

```sh
nix develop path:./dependencies/moergo-rmk --command \
  ./bin/moergo-control --usb version
```

At the exact pinned RMK commit, `dependencies/rmk/rynk/src/api.rs` within the
product repository produces four compiler errors:

| Error | Location | Detail |
| --- | --- | --- |
| E0252 | line 59, prior import line 34 | `LightingExtendedRuntimeConditionalScenesPage` imported twice |
| E0252 | line 59, prior import line 45 | `PutLightingExtendedRuntimeConditionalSceneChunkRequest` imported twice |
| E0425 | line 120 | `DeviceDataDescriptor` missing from scope |
| E0425 | line 125 | `DeviceDataRecord` missing from scope |

This is a host-library build defect, not an observed embedded runtime defect.
The successful firmware build and reported RMK host tests do not establish
that this separate host client builds. `just apply`, `just diff`, and `just
show` all invoke the same Cargo wrapper, so the prescribed restoration and
verification path is unavailable. They were not attempted as post-flash checks.
The safe-flash script also builds this control package before flashing.

Repair the imports on the owning RMK topic and regenerate/repin the assembly,
then build `moergo-control` and validate the runtime configuration before
resuming the flash sequence. No generated assembly files were edited here.
No embedded firmware fix is established by this session.

## Observations on existing hardware

The USB inventory showed one `16c0:27db` keyboard and no Go60 USB device or UF2
bootloader volume. After the wrapper failed, the existing, previously built
`dependencies/moergo-rmk/target/debug/moergo-control` was used **only for
read-only baseline commands**, inside the Nix shell. This is an older binary;
these observations do not validate the newly pinned client or firmware.

`--usb version` returned:

```text
moergo-control v0.1.0 (0fd8e8d3)
Rynk protocol: v0.2
firmware: config f08fe029-dirty / glove80-rmk v0.1.0 (0fd8e8d3) / RMK dff8ddf6
RMK: v0.9.0
device: MoErgo Glove80 (USB 16c0:27db)
serial: rmk:0.9.0
```

`connection status` reported USB connected, BLE slot 0 inactive, preferred BLE,
and typing routed to USB. `lighting replica-status` reported `IN SYNC`,
`link_up: true`, health `Healthy`, no pending acknowledgement, mismatch count
zero, and matching central/peripheral revision 8 and digests. The peripheral
status age was 132 ms. Both halves reported powered and output enabled; active
layer bits were 17, effective layer 4, default layer 0. This is protocol
telemetry, not visual inspection or a measurement of electrical power.

The old client's `config show` succeeded and its canonical output was saved as
the baseline. It was not applied or claimed to match the current source TOML.
`device-data` returned `device rejected Unimplemented`; do not infer absent
hardware capabilities from this older-client/firmware exchange. The right
half's firmware identity was not independently obtained.

## Recovery artifacts

Located the historical recovery pair in
`.worktrees/magic-runtime-crash/.handoff/candidate/recovery/` and verified their
UF2 structure and hashes against that directory's manifest:

```text
left  0x26000–0xd5800  0x9807b007
      91b6ba90033394d5cbe77f53748ff1d32097578ce339b8e0199b051b447b5368
right 0x26000–0x89f00  0x9808b007
      7abe61a879a3ec88f370ba39ee939b2ea2b0023dec869734820f6d3f9c6cd944
```

The manifest identifies product `d8a5510d2bad5a93936e1d6a0ef4a4dd8b278b43`,
RMK `ee4402398741e4d1abf1abaac4e4774948b7d259`, and configuration
`caa8b416b569f442624d72b3bf74f796567e822e` dirty. The adjacent historical
`evidence/magic-final-recovery.log` records a left flash followed by a stable
45-second health verdict. That is prior-session evidence, not a recovery
performed here. These are not an exact backup of the currently reported
`0fd8e8d3` firmware, and this session did not qualify the recovery right image.
Copies of both UF2s, manifest, and historical log are retained with this
session's evidence.

## Requested hardware coverage

| Check | Result |
| --- | --- |
| Glove80 rebuild and UF2 ranges | Passed locally as described above |
| Current central identity | Read successfully with existing older binary |
| Left-first flash, then matching right flash | Blocked; neither flashed |
| New version and right-half connection | Not tested; only existing-firmware link observed |
| One-time action storage reset | Not tested |
| Restore configuration; `just apply`, `just diff`, `just show` | Blocked by pinned host build; no mutations |
| Split unplug/replug | Not exercised |
| Split force both directions and missing-ack timeout | Not exercised |
| Half-duplex baud-derived deadlines | Not applicable to this configured BLE-only Glove80 split (`config/firmware.toml`) |
| Held modifier with pointing taps | Not exercised; pointing hardware not established |
| Pad button during mouse-key repeat | Not exercised; pointing hardware not established |
| Scenes and conditional rules across reboot | Not exercised |
| Powered-only authority/local scope on battery | Not exercised; both existing halves reported powered |
| BLE advertising backoff | Not exercised |
| Go60 | No Go60 USB device observed; no build or flash attempted; handoff overflow remains uninvestigated |

Resume with a buildable pinned host tool, then preserve the current runtime
snapshot and recovery pair, flash/qualify central first, flash the matching
right image, and restore/read back runtime configuration. Physical reconnect,
power-source changes, and any available pointing-input tests still require
hardware interaction; no software inference here substitutes for those tests.

## Local evidence and scope

Logs and snapshots are under `/tmp/glove80-qualification-20260907/`:
`rebuild.log`, `artifact-verification.log`,
`pinned-control-build-failure.log`, `preflash-version.log`,
`preflash-connection.log`, `preflash-replica.log`, `preflash-runtime.toml`,
and `preflash-device-data.stderr`. `prebuild/` holds the original local
candidate UF2s; `known-good-recovery/` holds the verified historical recovery
pair. The separate `recovery/` directory contains an earlier artifact search
candidate from the untracked legacy repository and is not the selected
historically documented recovery pair. These `/tmp` files are machine-local
and may be removed by system cleanup; essential findings are recorded above.

Only this report is committed. No source/configuration changes, vendored edits,
upstream work, or full test-suite runs were performed. Report whitespace was
checked with `git diff --check`; no Markdown-specific checker is provided by
the root project workflow.
