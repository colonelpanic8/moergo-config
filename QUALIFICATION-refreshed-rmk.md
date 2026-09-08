# Refreshed RMK qualification — jay-lenovo, 2026-09-08

## Current repair: reduced stack use and durable lighting restoration

The refreshed upstream line is retained. The changes repair the paired reset
loop and the lighting restoration defects found while qualifying it. The
brief failure/recovery history below distinguishes the earlier rollback from
the repaired firmware now installed.

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
The ten persistence failures recorded in report commit `9a02bd4` are resolved.

Crosshair and Rain were each selected through temporary runtime configurations,
read back, rebooted, and read back again with matching configuration and tuned
parameter values. Each test restored the source TOML afterward. With Reactive
selected, an explicit inactive Crosshair Arm hue change to 9 survived another
reboot; only that intentional difference appeared in the source diff.
`just apply` restored Arm hue 0, and a further reboot again matched the complete
source configuration with `IN SYNC` / `Healthy`, mismatch count zero. These
four additional reboots bring the final candidate total to seven. All changes
in these tests were runtime writes, with no additional flash.

A subsequent five-minute USB watch sampled every 0.5 seconds (600 samples),
observed device number 106 throughout, and recorded no absence or device-number
changes. Final readback still reported product `8226c521` / RMK `94fba2e5`,
`keyboard matches configuration`, and `IN SYNC`. This is USB/firmware telemetry,
not a visual, electrical, or RF measurement.

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
  `wire_values_locked` and `wire_frames_locked` failed. The baseline
  comparison against original RMK `ef978730` produced identical mismatches;
  these changes do not alter the wire protocol. They were not hidden by regenerating
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

The repair is proposed in [PR #16](https://github.com/colonelpanic8/moergo-config/pull/16).
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
`layout_compare.s`, and `qualified-stack.log`. Recovery artifacts and earlier
failure evidence remain available as listed below. No diagnostic reboot or
panic hooks were added to the published firmware.

## Failure history and recovery

The original connected firmware reported product `0fd8e8d3`, RMK `dff8ddf6`,
and configuration `f08fe029-dirty`. The initial refreshed host tool did not
compile: allocation feature guards/imports in the extended lighting pager
needed repair, and product keycode conversion referenced removed `OutputAuto`
and `DebugToggle` variants. Those fixes and their focused host tests are
carried in the published product chain. Unsupported retired actions are
rejected before keymap writes.

The first buildable refreshed pair (product `eb858baa`, RMK `c58577d4`) matched
the handoff's UF2 ranges: left `0x26000–0xd9300`, right `0x26000–0x8a200`.
Its central passed alone, then repeatedly disappeared/re-enumerated roughly
10–11 seconds apart after its matching right started. Both original
application images were recovered successfully, and runtime configuration
was restored and independently verified. That earlier intervention was a
rollback; the final installation described above is the repaired refresh.

Panic-recording, BLE runner-error trapping, and explicit reboot-trapping
variants still reproduced the paired loop. Retained diagnostics reported
increasing boot counts and reset reason `0x4`, without a recorded panic,
runner error, or explicit reboot caller. These observations and the excessive
static call-chain size directed the lighting stack repair; timing alone was
not used to diagnose memory exhaustion. Separating replica import/export
frames stopped the observed loop, including after diagnostic hooks were
removed. The later async snapshot change adds further central stack margin.

Exact original recovery images remain in
`.worktrees/qualification-refreshed-rmk/evidence/device-recovery/`. These are
application-only UF2s covering `0x26000–0xdc000`, derived without changing the
application payload. Do not substitute raw whole-device `CURRENT.UF2` files,
which also contain SoftDevice/storage regions.

```text
left-application.uf2
75b64c9a1588610c5fd8c546a5e547df31dfbf177221c8a21e555826b5a68393
right-application.uf2
4a0a77abf488388c187deb71e533f17c050ee893622dd0fd181ab13815d4ffc6
```

A second known-good recovery pair, qualified with the initial stack repair,
is retained in `.worktrees/qualification-refreshed-rmk/debug/clean-images/`
(product `aee5a663`, RMK `6d4a05da`). It has the earlier persistence limitation
but stable paired operation. This was the automatic recovery image supplied
during final central qualification; recovery did not trigger.

Earlier detailed reports remain in Git commits `759d4c3` (paired failure and
recovery) and `9a02bd4` (initial stack repair and persistence failures).
Diagnostic manifests and traces are under the local `debug/` evidence
folder. Only explicit source pins and this report were committed in the
outer repository; unrelated untracked files and runtime TOMLs were preserved.
