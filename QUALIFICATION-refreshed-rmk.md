# Refreshed RMK qualification — jay-lenovo, 2026-09-08

## Current repair: reduced stack use and durable lighting restoration

The refreshed upstream line is retained. The changes repair the paired reset
loop and the lighting restoration defects found while qualifying it. Older
observations below describe earlier candidates and recovery steps.

### What increased RAM usage

The layer/indicator condition addition (`c3325cf7`, with layer masks narrowed
by `e20591e36`) enlarged `ConditionSet` from 17 to 32 bytes and
`InternedRuntimeConditionalSceneCell` from 20 to 36 bytes. These sizes were
compiled for the ARM target from the actual old/new type declarations, not
estimated from the source. Each 100-cell table consequently grew by 1,600
bytes, repeated across live state, staging, and replica copies.

The original physical recovery application's reset code initializes 25,476
bytes of data and clears 130,352 bytes of BSS. The failing refreshed ELF has
27,188 bytes of data and 137,104 bytes of BSS: a combined increase of 8,464
bytes. This comparison uses the actual recovery UF2 and refreshed ELF; UF2
flash address ranges alone do not establish RAM exhaustion.

The previously documented failing direct call chain subtracted 100,956 bytes
from a 96,432-byte stack gap, before saved registers and interrupts. Besides
separating large replica import helpers from ordinary lighting commands, the
product now consumes exported replica snapshots in synchronous helpers;
large snapshots no longer travel through async return values.

The final candidate has text 707,260, data 27,188, and BSS 136,512 bytes.
`__sheap = 0x20027f80`, initial SP `0x2003fc08`: 97,416 bytes of stack gap.
The largest identified central direct-call path subtracts 82,164 bytes.
That analysis excludes the peripheral-only replica import arm and does not
bound saved registers, indirect/tail calls, or interrupts. No hardware fault
PC or stack high-water mark was captured. The static evidence and repeated
paired-failure/fixed-build comparison support excessive stack use as the
reset cause; they do not identify every byte of historical growth with one
commit.

### Lighting persistence repairs

Successful output-mode changes now have a durable record. Parameter writes
save the affected effect's parameter page, including inactive effects.
Startup loads those records on both supported central boards. New storage
variants are appended, preserving existing key/data variant ordering; no
Rynk wire changes or protocol-version changes were introduced.

Hardware testing exposed an additional startup edge case: Crosshair's board
defaults matched the source TOML, so the host did not resend them. Once
Reactive was saved as the selected effect, startup initialized inactive
Crosshair from generic library defaults. Startup now retains the board's
inactive defaults while preserving legacy selected/overlay tuning and giving
explicit durable parameter pages precedence.

### Published sources and installed artifacts

| Component | Commit |
| --- | --- |
| Owning lighting topic | `a7708fbe7f0cf379d7c6a5dc16c2c3175ae6448b` |
| RMK assembled | `94fba2e52c91fa691bcec3e71f27b10743151d8b` |
| Assembly recipe | `9555e710e7721a04c8ba8bfe256d97e795000c41` |
| Product used for hardware qualification | `8226c52173f84b574fd88ec430998e587e812a7c` |
| Published product, adding only the recipe gitlink | `401ea85500c238d61f735ffd676b4aa2a8e70869` |
| Outer product-pin commit | `de934bb` |

Both halves came from one clean product build using Rust 1.97.0, the absolute
outer `config/firmware.toml`, and outer provenance `9a02bd4b-dirty`. Every UF2
block's header, family, count, address order, contiguity, application bounds,
and manifest SHA were validated before flashing. Installed artifacts:

```text
left  0x26000–0xd9500  family 0x9807b007
      f79499ecfc5326249408be4d150c4ef5dc1ab35b99098b3042b65b1ff9ea1d69
right 0x26000–0x89f00  family 0x9808b007
      bde233e8c2bcbafe357b3ac60ce45a304bf18430a9f6d5f26d6d2191b3b8a867
```

These remain inside `0x26000–0xdc000`; compared with the handoff, the left end
increased by 512 bytes and the right end decreased by 768 bytes. Central was
flashed first with the previously qualified fixed-stack recovery available;
its 45-second USB watch passed before the matching right UF2 was flashed.
The central reports `config 9a02bd4b-dirty / glove80-rmk v0.1.0 (8226c521) /
RMK 94fba2e5`. The right's copied UF2 and healthy split telemetry are evidence;
its firmware identity was not separately read.

The recipe's public fork URL is restored. Locked builds reproduced tree
`0e982d205b292240c067838bb336f70a68ec29fc`. The published product adds only the
recipe gitlink to the hardware-tested product. Two builds of that published
product produced byte-identical UF2s, with the same address ranges and hashes
`f3a8e3a0af44998a14ca83e98e5215aa45c0becbfd957acf44fc3251122c2d71`
(left) and `f45febe7faf9b2e6758a52d83becd3f61e46142af830184b120129c5756e65d8`
(right). Those hashes differ from the installed images because product
provenance changed; those reproduced images were not flashed.

### Observed hardware checks

After the final paired flash, `just diff` identified the expected reset state.
`just apply`, independent `just diff`, and `just show` then succeeded. Three
consecutive software reboots, without reapplying configuration between them,
changed USB device numbers 99→100→101→102. After each reboot the complete
configuration matched and split telemetry reported `IN SYNC` / `Healthy`.
This includes the keymap, layers, scene tables, conditional rules, PoweredOnly
output mode, six nonstandard Crosshair defaults, and three Rain parameters.
The ten persistence failures documented in the earlier result are resolved.

Crosshair and Rain were each selected through temporary runtime configurations,
read back, rebooted, and read back again with matching configuration and tuned
parameter values. Each test restored the source TOML afterward. With Reactive
selected, an explicit inactive Crosshair Arm hue change to 9 survived another
reboot; only that intentional difference appeared in the source diff.
`just apply` restored Arm hue 0, and a further reboot again matched the complete
source configuration with `IN SYNC` / `Healthy`, mismatch count zero. These
four additional reboots bring the final candidate total to seven. All changes
in these tests were runtime writes, with no additional flash.

### Source verification and remaining coverage

- 125 relevant RMK lighting/storage tests passed, including inactive parameter
  writes and reopening simulated flash with an empty cache.
- All 16 split-lighting tests passed, including three new startup restoration
  regressions using the real PaletteFx source.
- Product `just parity-check` passed: host checks/tests and both boards' check
  builds. Outer `just check` passed both runtime configurations and compiled
  configuration parity. Changed RMK files and all changed board/test modules
  pass formatting checks.
- Native Rynk tests (39), its doctest (1), no-default-features, and WASM checks
  passed. Relevant RMK clippy passed with the documented pre-existing
  `Crc32::new`/`new_without_default` exception.
- Protocol tests retain the two pre-existing stale snapshots: 89 passed,
  `wire_values_locked` and `wire_frames_locked` failed. The previous baseline
  comparison below established those failures predate these repairs; these
  changes do not alter the wire protocol. They were not hidden by regenerating
  snapshots.
- The Go60 release build was attempted for the shared changes. Its existing
  flash overflow remains: `.data` exceeds FLASH by 13,420 bytes and
  `.gnu.sgstubs` by 13,440 bytes on the final candidate. No Go60 was attached
  or flashed; partitions and security features were not changed.

Physical unplug/replug, battery-powered authority/local VBUS behavior,
held-modifier pointing taps, and pad-button/keyboard-mouse-key overlap remain
unmeasured. This Glove80 build uses a fixed BLE split; force BLE/wired/auto
requests previously returned `Invalid`, so automatic transport acknowledgement
and timeout behavior and wired serial deadlines are unreachable here.
Software reboots and split convergence do not substitute for those tests.

The outer PR can validate the Glove80 repair, but master publication currently
runs `firmware-all`: the unrelated, pre-existing Go60 overflow still blocks
that release job. Do not interpret a green PR configuration check as a green
release build for both products.

New evidence is retained locally under
`.worktrees/qualification-refreshed-rmk/persistence/`: `qualified-*` logs,
`qualified-images/`, `reboot-*-runtime.toml`, `effect-checks.log`,
`preferences-tests.log`, `lighting-tests.log`, `host-checks.log`,
`protocol-tests.log`, `source-format.log`, `outer-check.log`,
`resolution-continue.log`, `reproducibility.log`, `recovery-layout.log`,
`layout_compare.s`, and `qualified-stack.log`. Earlier recovery artifacts and
failure evidence below remain available. No diagnostic reboot or panic hooks
were added to the published firmware.

## Earlier result: paired reset loop repaired, persistence still open

**The keyboard is running the refreshed firmware with a lighting stack-use
repair, not the recovery firmware.** Both clean candidate halves were flashed,
central first. They connected and reported `IN SYNC` / `Healthy`. A two-minute
USB watch observed no disappearance or device-number change. A subsequent
software reboot reconnected the split without restarting the loop.

This resolves the paired reset failure recorded below. Qualification also
found an outstanding lighting persistence defect: output mode and nine effect
parameters revert after reboot. Keymap, scene tables, and conditional rules
survived that reboot. Physical battery, unplug/replug, and pointing coverage
remain incomplete; this is not blanket qualification of every audit repair.

### What changed and why

The fix is confined to three files on the owning lighting topic:

- Make the mailbox command handler synchronous; its former async body had no
  awaits.
- Keep command dispatch boundaries out of line.
- Move replica export/import bodies into separate out-of-line helpers, keeping
  the large import temporaries out of ordinary commands on the central.

The command behavior, replica validation/application order, storage keys, and
wire protocol are unchanged. No reset suppression, BLE error trap, partition
change, or diagnostic firmware hook remains in the shipped source.

The evidence supports excessive **RAM stack use**, rather than Glove80 flash
exhaustion, as the reset cause. The failing original image fit its application
partition. A diagnostic build that reproduced the paired failure had 96,432
bytes between static allocations and the initial stack pointer. Disassembly
showed a direct call chain subtracting 100,956 bytes from the stack before
counting saved registers, indirect calls, or interrupts. That is a static
build finding, not a measured hardware stack high-water mark or a captured
fault address.

The failing diagnostic build's lighting mailbox frame subtracted 41,652 bytes. After the
repair, the clean central's general engine command frame subtracts 7,964 bytes;
its synchronous mailbox and service frames subtract 384 and 812 bytes. Large
replica import work remains separate and is used by the peripheral, which has
more RAM headroom. The central exports snapshots. The clean central has
98,064 bytes between `__sheap = 0x20027cf8` and `_stack_start = 0x2003fc08`;
ELF sections report text 704,704, data 27,188, and BSS 135,864 bytes. These
numbers are not a complete stack bound for every RMK configuration.

Hardware comparisons support the diagnosis: panic-recording, BLE runner-error
trapping, and explicit reboot trapping variants still looped after pairing.
Their readable diagnostics showed increasing boots and reset reason `0x4`,
without a retained panic, runner error, or explicit reboot caller. Timing alone
was not used to identify the cause. The lighting frame repair stopped the
observed loop both with those diagnostics and after removing them.

### Published sources and installed images

| Component | Commit |
| --- | --- |
| Owning lighting topic | `7d143d52416486dec1c714990df25674c115617b` |
| Published/tested RMK assembled | `6d4a05da37da00a6700ad3296aa982445f4f6fab` |
| Assembly recipe | `88b80ff0df89b114064cf784deb600682df0d0e1` |
| Product used for the flashed build | `aee5a66333ae7ec98c9261f5aa24bc8833d4ab3b` |
| Published product pin, adding only the recipe gitlink | `1a2b95a5075f8bb3f02109b84128759a427d659e` |
| Outer product pin | `fed442e1a22a287a4f019fb3b9e57753a1accb21` |

The topic, assembled branch, recipe, and product are pushed to their owning
forks. The upstream base remains `8b4d1b31`; the original split topic remains
`8158777f`. Locked assembly builds reproduced tree
`fb6077d7c3a77fd4ad1a202e5b4c7daa5b78424d`, including after restoring the
manifest's public fork URL. Generated commit IDs differ between rebuilds;
that tree is the reproducibility invariant. The product's last commit changes
only the assembly recipe gitlink, not firmware source. The installed firmware
reports the actual tested build's identity:

```text
config 759d4c30-dirty / glove80-rmk v0.1.0 (aee5a663) / RMK 6d4a05da
```

Built with Rust 1.97.0, the absolute outer `config/firmware.toml`, and outer
provenance. The product was clean. Validated all UF2 block headers, block
counts/order, contiguous addresses, family IDs, and application bounds before
flashing. Final installed artifacts:

```text
left  0x26000–0xd8b00  family 0x9807b007
      8e152c2a0800d60ba2cda322c9c8cb587d52c5ca01754813c3a30a4303959f57
right 0x26000–0x89c00  family 0x9808b007
      2565e2ecef696699c142a9688f172bb3fd870598fe8454b42775990f1e5da04c
```

Both are smaller than the failing handoff-range images. Exact original recovery
application UF2s listed below remain available. Each diagnostic trial and the
final candidate were flashed central first with recovery available and the
right staged in its bootloader. The final central passed the 45-second watch
before the matching right was flashed. A separate right firmware version was
not read; the right UF2 copy and healthy split telemetry are the observations.

### Hardware and runtime checks

The diagnostic repair pair accepted `just apply`; independent `just diff`
reported `keyboard matches configuration`, and `just show` exported it. Moving
to the clean build reset configuration again: its initial diff found 197
differences. Therefore the update itself is not evidence of persistence.
The clean firmware then passed apply, independent diff, and export as well.

At 07:31:55 UTC the clean pair had USB device number 89. A watch sampled it
240 times at 0.5-second intervals and observed no changes. Replica telemetry
reported a connected, healthy link with matching digests. Split transport
status reported `fixed (this board has no automatic wired/BLE policy)`;
forcing BLE, wired, and auto each returned `Invalid`. The two-way force and
missing-ack timeout paths are unavailable on this compiled BLE-only board;
no wired serial deadline claim is made.

For the reboot test, a temporary local `moergo-control` extension invoked the
existing Rynk `Reboot` command and waited for disconnect. The CLI patch and
executable are retained with evidence; its source edits were removed and were
not published. No firmware flash was used merely to reboot. USB changed from
89 to absent at 07:34:23 UTC and reappeared as 90 at 07:34:24 UTC. A 55-second
watch saw only this expected transition. Firmware identity was unchanged and
the replica returned `IN SYNC` / `Healthy`.

Independent post-reboot diff found exactly these differences:

| Setting | Source / before reboot | After reboot |
| --- | --- | --- |
| Output mode | PoweredOnly | AlwaysOn |
| Crosshair: Arm hue | 0 | 172 |
| Crosshair: Arm width | 11 | 8 |
| Crosshair: Crosses | 1 | 4 |
| Crosshair: Duration x10ms | 90 | 16 |
| Crosshair: Key hue | 173 | 16 |
| Crosshair: Pulse width | 170 | 56 |
| Rain: Drops | 11 | 6 |
| Rain: Spawn x10ms | 5 | 30 |
| Rain: Trail | 161 | 128 |

No keymap, layer, scene, or conditional-rule differences were reported. This
supports persistence of those managed records through this software reboot;
it is not a visual LED check. The ten settings above need a separate firmware
persistence investigation. Their failure was observed on this candidate; it
was not established whether the stack repair introduced it. Reapplying the
TOML is a runtime workaround, not a persistence fix.

After the reboot test, `just apply` restored and verified the source TOML;
independent `just diff` reported `keyboard matches configuration`, and
`just show` exported the final state. The first replica read was `RESYNCING`
while finishing the update. A later read reported `IN SYNC`, revision 10,
`Healthy`, `durable_dirty: false`, no pending acknowledgement, and matching
central/peripheral digests. The final version read still identifies the clean
`aee5a663` / `6d4a05da` firmware. These settings are restored now; the ten
persistence failures can recur on another reboot.

No physical battery-source change or unplug/replug was performed. No attached
pointing pad was established or pad/modifier/button interaction exercised.
The powered-only authority/local VBUS distinction and pointing fixes remain
unqualified. Software reboot/split convergence does not replace those tests.

### Software verification and remaining build limitations

- Focused RMK lighting suite on the clean assembled tree: 123 passed,
  648 skipped. Includes replica and host loopback coverage.
- Changed Rust files pass formatting. Whole-crate formatting reports an
  existing unrelated difference in `keyboard.rs:1386`, left untouched.
- RMK library clippy reports an existing `new_without_default` on `Crc32`.
  With that single lint allowed, the same library check passes. No source
  suppression was added.
- Rynk: 39 library tests and one doctest passed; its workspace also ran 43
  KLE and two USB tests successfully. Native/no-default checks and the
  explicit Rynk WASM check passed.
- Protocol tests again report 89 passed and the same two known stale snapshot
  failures (`wire_values_locked` and `wire_frames_locked`). Protocol sources
  are unchanged by this three-file repair; prior baseline comparison is below.
- Product `just parity-check` passed. Outer `just check` passed both firmware
  configuration parity checks and both runtime TOML validations.
- Both Glove80 halves built and linked. The Go60 release bundle was attempted
  with its own outer firmware configuration and still cannot link: `.data`
  overflows FLASH by 10,604 bytes and `.gnu.sgstubs` by 10,624 bytes. No Go60
  was attached/flashed; no Go60 partition or feature changes were made.

Evidence is retained under the ignored directory
`.worktrees/qualification-refreshed-rmk/debug/`: `clean-images/` contains the
installed ELF/UF2 pair and manifest; `clean-*-tests.log`, `clean-*-check*.log`,
`clean-*-lint.log`, `clean-layout.log`, `clean-disassembly.txt`, and
`clean-stack.log` hold build evidence. Flash logs are `left-clean-flash.log`
and `right-clean-flash.log`; runtime/watch logs include `clean-paired-status.log`,
`clean-apply.log`, `clean-after-diff.log`, `clean-usb-watch.log`,
`post-reboot-checks.log`, `post-reboot-show.toml`, and `reboot-usb-watch.log`.
Final readbacks use the `final-*` prefix. `inspect_stack.py` states its static
analysis limitations. Failed diagnostic trials are retained in
`diagnostic-images/`, `diagnostic-cause-images/`, `runner-images/`, and
`trap-images/`; `replica-images/` is the successful diagnostic repair pair.
Their manifests identify every image actually flashed. `low-inline-images/`,
`sync-images/`, and `outline-images/` were built but not flashed. No diagnostic
source changes were propagated to the outer pin.

---

## Historical result after host repair

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
