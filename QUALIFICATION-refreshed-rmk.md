# Refreshed RMK qualification — jay-lenovo, 2026-09-07

## Current result after host repair

**Host tooling repaired; paired firmware failed hardware qualification. Both
original application images are restored, and live runtime configuration
matches `config/glove80.toml`.** Both candidate halves were
flashed, left first. The left was stable with the right in its bootloader, but
began repeatedly disconnecting and re-enumerating after the matching right
image started. No runtime apply ran on the candidate firmware; runtime
restoration and source configuration apply succeeded after recovery.

The user subsequently requested repair of the host build blocker described in
the historical preflight below. The repair is published through the owning
topic and assembly, with no hand-commits to generated RMK output:

| Component | Repaired commit |
| --- | --- |
| RMK lighting topic | `7dbe32098d5d773068349f83ddb412eed91b12d2` |
| RMK assembled | `c58577d40ae48dabf4110ae3edeff902b3fe404e` |
| RMK assembly recipe | `0202409136b2c1195179e806f98d76690181e04d` |
| moergo-rmk | `eb858baa9d572e016aa63c6087604a1bc7e6acae` |
| Outer configuration pin commit | `7608079d13150cc672d8b50ea2af6a1edccb4a7a` |

The topic fixes allocation feature guards for the extended lighting pager and
its imports. Tracked merge resolutions preserve device-data imports; the
lighting coherence fixup removes duplicated imports. An obsolete formatting
hunk in the persistent-layer fixup was recaptured without changing its
resulting firmware content. Locked rebuilds reproduced tree
`42013d8bab7b94129c781a39b8a9a5854037d971`.

Building the repaired client exposed another compile failure: the product's
keycode converter still referenced upstream's removed `OutputAuto` and
`DebugToggle` variants. Those conversions and the unsupported output-auto
name were removed. Focused CLI tests verify rejection of the retired codes
before keymap writes and acceptance of supported output/bootloader actions.
Relative to the original assembled tree, the final RMK change is confined to
host file `rynk/src/api.rs`; embedded behavior was not patched.

### Software verification

- Installed `./bin/moergo-control` builds; `just check` passes both firmware
  configuration parity checks and both runtime TOML validations.
- Rynk native and no-default-features checks pass; 39 library tests, one
  doctest, and native library clippy pass.
- Rynk WASM and product configuration WASM type checks pass.
- Product keycode conversion tests: 2 passed; keycode-name tests: 6 passed;
  CLI keymap tests: 8 passed. Formatting and production clippy pass, including
  the CLI's test targets.
- Protocol tests: 89 passed, two stale snapshots failed. Running those same
  two tests on original RMK `ef978730` produced identical mismatches, verified
  by comparing the actual snapshot text. Differences are maintenance and
  Control/GUI keyboard-action discriminants and consumer-key discriminants
  (including the auto-mouse exemplar). No protocol bytes were changed by this
  repair; snapshots were not regenerated to hide the pre-existing failures.
- An additional all-targets lint invocation found an existing `clone_on_copy`
  in the product configuration tests at `config.rs:4192`. It was left alone;
  the narrower checks listed above passed. No full RMK suite was rerun.

### Images actually flashed

`just firmware` succeeded with the repaired pins and outer configuration
provenance. The product source was clean; outer `dirty = true` reflects the
same pre-existing untracked files. Validated UF2 block headers, counts,
contiguous addresses, family IDs, manifest hashes, and provenance before
flashing. Both ranges still match the original handoff:

```text
left  0x26000–0xd9300  0x9807b007
      eb71d28b5a0f7e91c113329a5f59dcc389e05a74cfd3d924d07ed72ae77fe700
right 0x26000–0x8a200  0x9808b007
      a61ee7d7e41705f3a2e204e77d207f5feb7e6b616bb8ff453fa805e44b418254
```

The right accepted the bootloader request and exposed `GLV80RHBOOT` over USB.
It was staged in the bootloader before changing the left image, avoiding a
dependence on mixed-version split messages to enter the right bootloader.
The left was flashed first using `bin/glove80-safe-flash`, with recovery ready.
At 06:20:10 UTC on September 8 it reported:

```text
config 7608079d-dirty / glove80-rmk v0.1.0 (eb858baa) / RMK c58577d4
```

The left passed the script's 45-second USB stability watch and answered a
full runtime diff. The diff found 197 differences from `config/glove80.toml`,
consistent with the expected storage reset; it did not change the source or
keyboard. `device-data` still returned `Unimplemented` on the new firmware.
The right image was then copied successfully, logged at 06:21:21 UTC. Its
bootloader USB volume disappeared. The right's running firmware identity was
not independently read back, and no healthy paired replica report was obtained.

### Observed hardware failure

After the right started, central USB port `3-1` repeatedly disconnected and
re-enumerated. Kernel records show disconnects at local times 23:21:52,
23:22:03, 23:22:13, 23:22:24, and repeatedly thereafter, followed by enumeration
about one second later. This is an observed reset/re-enumeration pattern;
neither a crash backtrace nor a watchdog/reset-reason register was obtained.
The timing alone does not prove a watchdog fault or RAM exhaustion.

The first paired `lighting replica-status` failed with no Rynk USB device
found. A subsequent attempt failed to establish a session with an I/O error.
The command sequence stopped before `just apply`; there is no apply log because
the apply command was never reached. Therefore configuration restoration and
all later behavioral checks remain unqualified. The observed trigger is
starting the freshly flashed right half after the new central had been stable
with the right in bootloader; the underlying firmware cause is not established.

### Recovery and remaining coverage

Before flashing, saved a fresh canonical runtime snapshot with the compatible
old client as `preflash-current-backup.toml`. The repaired client could read
the old firmware identity and maintenance-unlocked state, but could not decode
its old action layout; the compatible older executable is preserved as
`pre-repair-moergo-control` in the evidence directory.

Also read `CURRENT.UF2` from **both physical bootloader drives** before any
candidate flash. The originals cover `0x1000–0xea000`, including SoftDevice
and storage, so they are retained as backups rather than flashed wholesale.
Derived recovery UF2s contain only the application partition
`0x26000–0xdc000`, preserving its payload bytes exactly and regenerating block
sequence/count fields. Their hashes are:

```text
left-application.uf2   75b64c9a1588610c5fd8c546a5e547df31dfbf177221c8a21e555826b5a68393
right-application.uf2  4a0a77abf488388c187deb71e533f17c050ee893622dd0fd181ab13815d4ffc6
```

Remote right-bootloader requests failed during the reset cycle; the recovery
script exhausted its retries without entering that bootloader. Central
recovery then succeeded and passed its 45-second USB watch. The recovered
central again reported product `0fd8e8d3`, RMK `dff8ddf6`, and outer
`f08fe029-dirty`. It could communicate with the right; a subsequent automated
right recovery succeeded. Physical assistance was requested during the reset
cycle but was not needed to complete recovery.

The compatible old client applied the saved pre-flash snapshot with `--exact`,
reported `applied and verified`, and an independent diff reported `keyboard
matches configuration`. Its replica readback reported `IN SYNC` and both
halves connected. A 45-second USB watch observed no disappearance or USB
device-number change.

Comparing that restored snapshot with the current source TOML found 52
pre-existing differences: two layer-5 key bindings, one layer-5 scene cell,
and 49 conditional-rule entries. Applied `config/glove80.toml` through the
compatible client as runtime configuration, then independently diffed and
exported it. Apply verified successfully and the final diff again reported
`keyboard matches configuration`. Thus the final state uses the recovered
firmware with the requested source TOML, while the exact original runtime
snapshot remains available separately.

Final checks after source apply reported replica revision 10, `IN SYNC`,
`link_up: true`, `Healthy`, no outstanding acknowledgement, zero mismatches,
and a peripheral status age of 141 ms. Another 45-second USB watch, ending
06:32:29 UTC, observed the same USB device throughout. The final central
version readback again identified the recovered `0fd8e8d3` application.

The normal wrapper now builds the repaired host tool, but its newer action
decoder cannot read the recovered older firmware's keymap. Until firmware
qualification is repaired, the saved compatible executable can manage that
keyboard without another flash:

```sh
nix develop path:./dependencies/moergo-rmk --command \
  ./.worktrees/qualification-refreshed-rmk/evidence/pre-repair-moergo-control \
  --usb config diff config/glove80.toml
```

Split reconnect, forced-transport acknowledgement/timeout, reboot persistence,
powered-only battery scope, and pointing interactions were not exercised after
the failure. The Glove80 configuration is fixed BLE split, so wired timing is
not applicable. No Go60 USB device was observed and no Go60 firmware work was
performed. This paired build must not be marked hardware-qualified.

All follow-up logs were recorded in `/tmp/glove80-qualification-20260907/`, including
`installed-host-check.log`, `final-host-checks.log`, `product-host-checks.log`,
`product-host-final-checks.log`, `product-wasm-check.log`,
`protocol-final-checks.log`, `baseline-protocol-snapshots.log`,
`assembly-publish-locked.log`, `repaired-firmware-build.log`,
`repaired-artifact-verification.log`, `left-repaired-flash.log`,
`right-repaired-flash.log`, `new-central-version.log`,
`new-central-before-apply-diff.log`, `paired-failure-kernel.log`,
`paired-failure-replica.log`, both initial recovery-flash logs,
`right-recovery-after-left.log`, `recovery-runtime-apply.log`,
`recovery-verified-diff.log`, `recovery-source-apply.log`,
`recovery-source-verified-diff.log`, `recovery-source-verified-show.toml`,
`final-recovery-replica.log`, `final-recovery-version.log`, and
`final-recovery-usb-watch.log`.
`repaired-candidate/` holds the flashed pair; `device-recovery/` holds both
device backups, application recovery UF2s, bootloader identities, and extraction
verification. A complete copy is retained under the ignored local directory
`.worktrees/qualification-refreshed-rmk/evidence/`, so the recovery images,
compatible client, snapshots, and logs survive `/tmp` cleanup. Neither those
artifacts nor unrelated working-tree changes are included in this report commit.

---

## Historical initial preflight, before the requested repair

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
