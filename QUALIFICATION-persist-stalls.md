# Persist-stall qualification — jay-lenovo, 2026-09-14

Follow-up to `HANDOFF-persist-stalls.md`. Hardware: the attached Glove80,
left half on USB (`/dev/hidraw3`), right half on BLE only. The diagnosis in
the handoff is confirmed on hardware (section 1); the 24-sector partition
alone removes the stalls (section 2); the firmware fixes are folded, built,
flashed and measured (sections 4 and 5).

## 1. Baseline on the 8-sector firmware

Device before any change: `config 70fb3782-dirty / glove80-rmk 050a2cac /
RMK dd592a3d`, storage `0xec000` + 8 sectors (predates moergo-rmk `22ab770`).

`moergo-control keymap persist-probe --rounds 40 --writes` (each round writes
one item, then waits for the flash queue to drain behind a layer-metadata read):

| Rounds | Queue drain | One-item persist |
| --- | ---: | ---: |
| 1-25, 27-40 | 11-15 ms | 83-97 ms |
| 26 | 12 ms | **29.785 s** |

One write in forty paid a sequential-storage page migration of ~30 s while
every other write took under 0.1 s. That is the bimodal pattern the handoff
predicted (`sequential-storage-8.0.1/src/map.rs`, `migrate_items`, driven
through MPSL flash timeslots) and the same ~45 s-per-page stall the earlier
restore agent measured from the outside.

`config diff config/glove80.toml` found three drifted cells (layers 3, 4, 5
at r0,c0: keyboard `KC_F13`, file `KC_TRNS`). The earlier agent had reported
a match right after its restore; the kernel log shows the keyboard did not
re-enumerate between that restore and this check, so this drift is not a
reboot exposing lost writes. It is unexplained and was small enough to just
re-apply. `config apply` wrote those 3 cells with a longest flash wait of
0.2 s.

Silent-loss check: `moergo-control reboot --yes` (new command, see below),
then `config diff` again: `keyboard matches configuration`. Nothing that
apply wrote was dropped on the 8-sector store at that moment.

## 2. The 24-sector firmware

Built with `just firmware` from glove80-config `838298a0` (dirty: the outer
`config/firmware.toml` and `config/go60-firmware.toml` had to be synced to
the crates' 24-sector `[storage]` block first, or `firmware-config-check`
fails) and moergo-rmk `84e0ff9b` (clean), RMK `dd592a3d`. Left half UF2
`eba8344b…`, ends `0xd5200`; right half `6cdef82b…`, ends `0x86800`.

Only the left half was flashed (`bin/glove80-safe-flash … --recover
~/glove80-priority-test/glove80-recovery-lh.uf2`, verdict `stable`). The
right half is not on USB, so its bootloader volume cannot mount from here;
the firmware-relevant diff since its build is only the partition and
`memory.x`, so the halves remain protocol-compatible. Flash the right half
when a cable is available.

After the store wipe: `config diff` reported 197 differences (all lighting
conditional rules and 41 keymap cells against compiled defaults).
`config apply config/glove80.toml`:

```
persisted 41 cell(s) in 29 write(s) across 6 layer(s); waited 5.1s for flash in total, longest 0.3s (layer 0, offset 6)
applied and verified config/glove80.toml
```

Whole apply, lighting included: 15 s wall clock. Then the probe again:

| Rounds | Queue drain | One-item persist |
| --- | ---: | ---: |
| 1-40 | 5-7 ms (max 55 ms) | 75-89 ms |

No migration in 40 writes plus the apply's 29. Reboot, then `config diff`:
`keyboard matches configuration`.

## 3. Host changes (moergo-rmk master, pushed)

| Commit | Change |
| --- | --- |
| `7e15db5` | `moergo-control reboot [--yes]`: plain `Reboot` over Rynk, waits for the USB disconnect like the bootloader path. Needed for the silent-loss check. |
| `84e0ff9` | Recovered from the mislabelled jay-lenovo stash: the ZMK importer emits `PositionCombo` from key positions instead of resolving them to actions (which lowered transparent cells to `KC_TRNS` triggers that can never fire and collapsed same-action cells). 20 `moergo_import` tests pass. |
| `afc27cc` | Workspace `exclude` for `dependencies/rmk/.worktrees` so topic-branch worktrees build. |
| `48a4c7d` | `flash_channel_size = 16` on both boards (mirrored in the outer firmware TOMLs). |
| `7c10188` | `config apply` retries a page the firmware answers `Busy`, with a 100 ms delay for up to 300 s, and counts retries in the persist summary. |

## 4. Firmware fixes (rmk fork topic branches, pushed)

Both are based on the assembly's upstream base `8b4d1b312` and pass the
base's four CI feature sets under nextest plus clippy.

- `fix/rynk-flash-backpressure` (`d3ab17eda`): `SetKeymapBulk` and
  `SetKeyAction` wait at most 500 ms for the flash queue to have room for the
  whole page (`rmk/src/host/context.rs`, `wait_for_persist_room`) and answer
  `RynkError::Busy` with nothing applied otherwise. A page longer than the
  queue keeps streaming as before, so hosts that page by payload size still
  work. The `rynk` client caps keymap write pages at four cells and resends
  on `Busy` (`write_pages`).
- `fix/storage-sparse-keymap-init` (`c098056a2`): a fresh store no longer
  seeds every compiled keymap and encoder cell; boot already overlays stored
  cells on the compiled keymap (`rmk/src/host/storage.rs`, `read_keymap`).
  Removes ~22 KiB of permanently live items on the Glove80.

The `KeyPointerCache` idea (handoff 4c) was not pursued: it only shortens
read scans, and after 4b the store is far from full.

## 5. Assembly and final measurement

Both entries were appended to `rmk-assembly` (`30d0a49`, pushed) and built
with `fork-assembler build`; three rerere pairs record the adjacency
conflicts with the lighting stack (`rmk/src/host/context.rs`,
`rynk/src/api.rs`, `rmk/src/storage/mod.rs`). `build --locked` reproduces
tree `997ca98e`. The assembled commit `01178c2f` is published on the fork
as `assembled-persist-stalls`; moving `assembled` itself is a force push
that was refused in this session and is left to the maintainer, along with
tagging the old `dd592a3d2` if it should stay reachable. moergo-rmk
`31ba39c` (pushed) pins rmk `01178c2f` and the assembly.

Firmware built from that pin (glove80-config `838298a0`-dirty, moergo-rmk
`31ba39cf`, RMK `01178c2f`): left `068f1bfd…` ends `0xd5200`, right
`b3f7b5f1…` ends `0x86600`. The build's four `glove80_lh` warnings predate
these changes. Left half flashed (`stable`); right half again not on USB.

Measurements on the fixed firmware, all with the CLI read timeout at 40 s
so a real stall would fail rather than wait:

| Step | Result |
| --- | --- |
| `config apply config/glove80.toml` after the store wipe | 41 cells in 29 writes, flash wait 3.1 s total, longest 0.3 s |
| `config apply config/tailorkey-v52-bilateral.toml` | 635 cells in 184 writes across 12 layers, plus 9 combos and 84 macro bytes; flash wait 54.1 s total, longest 1.2 s; 61 s wall clock; no `Busy` |
| `config apply config/glove80.toml` back | 388 cells in 106 writes, 6 layer names; flash wait 29.0 s total, longest 0.4 s; 34 s wall clock; no `Busy` |
| `persist-probe --rounds 220 --writes` (twice) | one-item persist min 72 ms, mean 85-88 ms, max 143-145 ms |
| lighting-only apply (12 rules changed, then restored) | 11.6 s and 7.2 s |
| `reboot` then `config diff` (three times) | `keyboard matches configuration` |

About 1,700 items went into the store across these runs, several 4 KiB
pages' worth, so page boundaries were crossed repeatedly; the worst single
write anywhere was 1.2 s, against 29.8 s on the 8-sector firmware. The
`Busy` path was never taken because the queue never held more than a page's
worth of work. Its firmware side has a unit test for the room decision only
(`persist_room_boundaries`); the handler's bounded wait has not been
exercised on hardware. The host retry is covered by the moergo-control
tests.

One event needs an honest note: the very first apply after this flash hung
for ten minutes after printing its keymap summary. During that apply I ran
`moergo-control config validate config/glove80.toml` from another shell,
which opens the same hidraw device to resolve the layer names and thereby
ran a second Rynk session on the keyboard; the apply's next reply went to
it. Every later apply, including the lighting-only one that reproduces the
same write mix, completed normally. Do not run two `moergo-control`
processes against the keyboard at once, `validate` included.

## 6. Follow-up, later on 2026-09-14

- **Right half** flashed with `dist/glove80-rmk-0.1.0-rh.uf2` (`b3f7b5f1…`)
  through `bin/glove80-safe-flash --peripheral`; the cable turned out to be
  attached. It reconnected (`lighting read`: right half connected) and
  `config diff` still matched `config/glove80.toml`. Both halves now run
  moergo-rmk `31ba39cf` / RMK `01178c2f`, released as v2026.09.14.2.
- **`assembled` on the fork** now points at `01178c2f`; the previous
  `dd592a3d2` is kept as tag `assembled-dd592a3d2`.
- **Rynkbench** was the piece Moosy actually uses, and it had the same
  failure as the old CLI: an import wrote each changed cell under a 5 s
  request watchdog that closes the link on timeout, so on the 8-sector store
  every import died at the first page migration with the lighting section
  never written. Rynkbench `346053b` (deployed to GitHub Pages) is re-pinned
  to moergo-rmk `31ba39c` / RMK `01178c2f`, pages whole-keymap writes at
  four cells, resends keymap writes the firmware answers `Busy`, and gives
  flash-bound requests a 120 s budget so older firmware slows an import
  instead of dropping it. Its `nix/rynk-wasm-Cargo.lock` keeps the
  wasm-bindgen family at 0.2.126 because the pinned nixpkgs ships no 0.2.128
  CLI.
- **Moosy**: `config/tailorkey-v52-rynkbench-repaired.toml` has
  `output_mode` back at his original `always-on` (the LED-power theory was
  wrong) and validates against the fixed firmware. The uniform warm-white
  board in his photo is most likely the compiled default lighting (Crosshair
  on the Amber palette), since no import ever reached its lighting phase;
  whether his pale Cursor-layer palette reads as white on the LEDs is still
  untested. He needs to flash v2026.09.14.2 on both halves (first boot wipes
  the store), hard-refresh the editor, and load the file again.
- Still open: mouse layer scaling for TailorKey's three mouse layers (the
  stock-parity decision) and the three unexplained `KC_F13` cells from
  section 1.

## 7. Moosy's follow-up, later on 2026-09-14

- **The green key** was the firmware's compiled output-mode indicator
  (`e30a6f2`, 2026-07-30): three compiled conditional scenes lighting LED 31
  (the A key) whenever layer index 2 is active, green/red/blue for
  always-on/always-off/powered-only. `config/glove80.toml` had already moved
  that indicator to runtime rules on T, so the compiled copy only survived by
  oversight, and on a layout with Autoshift in slot 2 it lit the A key with no
  way to remove it. Dropped from the stock board file and
  `config/firmware.toml` (moergo-rmk `9cf4d77`). Go60 never had it.
- **Factory reset.** Rynk `StorageReset(Full)` was implemented in firmware but
  exposed nowhere. `moergo-control storage wipe [--yes]` (moergo-rmk
  `0255975`) and a "Wipe stored settings" action in Rynkbench's Danger zone
  (rynkbench `8f78869`) send it. The firmware's storage task erases the
  partition and reboots the keyboard itself; the first attempt sent a host
  reboot right after the reset, which won the race and left the store intact.
  With only the reset sent, the erase plus reboot took about fifteen seconds
  on this Glove80: an applied config showed 197 differences afterwards,
  `config apply` restored it, and it survived a reboot. `LayoutOnly` still
  answers `Unimplemented`; a wipe also drops Bluetooth pairings.
- Both halves flashed from moergo-rmk `114a7fb0` (the CLI commit before its
  amend to `0255975`; the board crates are identical between the two), left
  `e115ad7d…`, right `505b4b09…`, same address ranges as before.

