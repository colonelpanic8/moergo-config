# Handoff to jay-lenovo: `config apply` flash stalls on the Glove80

Written 2026-09-14 by the Fable 5.1 session on ryzen-shine (session name
`glove80-config-cb`). Everything below is pushed. You have the Glove80 on USB
(`/dev/hidraw0-2`); the ask is to verify the diagnosis on hardware, flash the
partition change, and finish the firmware-side fixes.

## Pins (all pushed)

| Repo | Commit |
|---|---|
| `colonelpanic8/glove80-config` master | `838298a0` |
| `colonelpanic8/moergo-rmk` master | `f429fc2` (on top of upstream `349e95f`) |
| `colonelpanic8/rmk` branch `assembled` | `dd592a3d2` (recorded by moergo-rmk) |
| `colonelpanic8/nrf-sdc` | `2dfeabc` (timeslot flash overstay fix) |

Your checkout is at `cdc9e0a` with local modifications: `config/tailorkey-v52-bilateral.toml`,
untracked `HANDOFF-refreshed-rmk.md`, `config/tailorkey-v52-rynkbench-repaired.toml`,
`dependencies/glove80-rmk/`, and modified content inside `dependencies/moergo-rmk`.
Look at `git -C dependencies/moergo-rmk status` before pulling; stash or commit
what is there, then:

```sh
cd ~/Projects/glove80-config
git pull --ff-only origin master        # -> 838298a0
git submodule update --init dependencies/moergo-rmk   # -> f429fc2
git -C dependencies/moergo-rmk submodule update --init  # rmk -> dd592a3d2
```

## The diagnosis (verified in code, partly on hardware)

Symptom: `moergo-control config apply` stops getting replies mid-keymap, later
Rynk sessions fail with `io error: TimedOut`, the keyboard keeps typing, and it
recovers by itself after minutes. It looked like a wedged Rynk service. It is
not.

1. **Blocking flash channel.** `SetKeymapBulk` awaits
   `FLASH_CHANNEL.send()` per cell (`rmk/src/host/context.rs`, `set_action`,
   also used by `SetKeyAction`). The channel is 4 deep (`flash_channel_size`
   default, `rmk-config/src/lib.rs`). The Rynk session is one future
   (`rmk/src/host/rynk/mod.rs`, `run_session`), so while a handler is parked on
   the channel it reads no USB OUT reports; the kernel's interrupt-OUT write
   times out on the host. Vial's path deliberately uses `try_send` and drops
   persists instead (`context.rs`, `try_set_action_flat`), which is lossy, not
   a fix.
2. **Slow consumer: sequential-storage page migration.** When the current
   page fills, `store_item` closes it and migrates the previous page's live
   items inline (`sequential-storage-8.0.1/src/map.rs:520-556`,
   `migrate_items`). Every migrated item costs two flash writes plus one page
   erase, each an MPSL radio timeslot (`nrf-mpsl/src/flash.rs` in the nrf-sdc
   fork: 7.5 ms write slot + 4 ms slack, 5 ms partial-erase steps). A slot that
   long never fits the 7.5 ms split-link interval at normal priority, so every
   write goes BLOCKED then HIGH. Tens of seconds per migration.
3. **The store was ~90 % full by design.** `initialize_storage_with_config`
   (`rmk/src/storage/mod.rs:1477-1485`) re-stores every cell of all 16
   compiled layers whenever the build hash changes (`check_enable`,
   `:1570-1577`): 1344 items of 16-24 B, ~22 KiB, before any user data.
   sequential-storage keeps one sector erased as buffer, so 8 sectors gave
   ~28.6 KiB usable. Lighting shards (two generations kept), morses, names,
   bonds pushed it near capacity, so each migration freed only a few hundred
   bytes and GC fired every ~32 writes. Past full, `store_item` returns
   `FullStorage`, the storage loop logs "Storage is full" and drops the write
   silently (`storage/mod.rs:1901-1904`); the host is never told.

Ruled out on the way: matrix holes (`HOLES` in moergo-config) are ordinary
cells; a permanent deadlock (it recovers); radio contention alone (powering
off the right half did not help because the central then scans 30 ms of every
100 ms and advertises); reply-window `Busy`.

Hardware data so far (from the debugging box): with 47-key pages the first
page stalled; with 4-key pages page 8 (~32 cells) stalled; a 120 s host
timeout rode through and the restore completed. Recovery without replug was
observed after a few minutes idle.

## What is already fixed and pushed

- **Host (`moergo-rmk` f429fc2, `crates/moergo-control`)**: `config apply`
  writes only cells that differ, in pages of 4 (`MOERGO_PERSIST_BATCH`,
  `MOERGO_WRITE_WHOLE_LAYERS=1`), and after each page waits on
  `GetLayerMetadata`, which the firmware serves FIFO behind the queued writes,
  so its latency is the persist time. Waits >= 2 s print as they happen and a
  summary follows. New `keymap persist-probe [--rounds N] [--writes]`. USB
  read idle timeout 300 s (`MOERGO_RYNK_READ_TIMEOUT_SECS`).
- **Firmware config (`moergo-rmk` 22ab770)**: `[storage] start_addr = 0xdc000,
  num_sectors = 24` on Glove80 and Go60 (96 KiB; app is capped at 0xdc000 by
  memory.x; Go60 image ends 1 KiB below it). Type-checked for both boards on
  ryzen-shine. **Not flashed anywhere yet.** First boot erases the store, as
  every rebuild already does, so apply the runtime config afterwards.
- Docs: `docs/layout-tools.md`, "Persistence: what `config apply` waits for".

## What to do next, in order

1. **Baseline on the current firmware** (before flashing): build the CLI from
   `dependencies/moergo-rmk` (`cargo build -p moergo-control`), then run
   `moergo-control keymap persist-probe --rounds 8 --writes`. Record numbers.
   Then `config diff config/glove80.toml`, then `config apply` and keep the
   printed persist waits and summary. This is the hardware confirmation of the
   page-migration story: waits of 5-30 s every few dozen cells.
2. **Check for silent loss**: after apply, reboot the keyboard (unplug with
   batteries out, or `moergo-control bootloader` then reset) and run
   `config diff` again. Any drift = writes that hit "Storage is full".
3. **Flash the 24-sector firmware** with `bin/glove80-safe-flash` (both
   halves, left half is the central and owns storage). Expect a wiped store.
   Apply the config, repeat the probe. Persist waits should drop to well under
   a second and migrations should be rare (every ~180 cells) and short.
4. **Firmware fixes on a `fold/*` topic branch** (never commit in
   `dependencies/rmk`; see `~/Projects/rmk-assembly`, `manifest.toml`, and the
   memory note `rmk-fork-fold-assembly` in `~/.claude-dean/projects/...`):
   a. `SetKeymapBulk`/`SetKeyAction`: check `FLASH_CHANNEL.free_capacity()`
      before applying anything and answer `RynkError::Busy` instead of
      blocking, so the session stays answerable; the host's bulk-read path
      already retries `Busy` (`rynk/src/api.rs` around `fetch` lanes), extend
      that to writes.
   b. `initialize_storage_with_config`: stop materialising cells equal to the
      compiled default (or store only layers the toml actually defines). That
      alone removes ~21 KiB of permanently live items.
   c. Consider `sequential_storage::cache::KeyPointerCache` instead of
      `Cache::new_uncached()` (`storage/mod.rs:957`); it only cuts the read
      scans, not the timeslot cost.
   d. Optionally raise `flash_channel_size` in the board toml (each slot is
      ~264 B of RAM; 16 is cheap) so a 4-cell page never blocks even when a
      lighting persist is queued ahead of it.
5. Record results in `QUALIFICATION-refreshed-rmk.md` style and push.

## Constraints and traps

- `dependencies/rmk` is the generated `assembled` branch: read-only. Firmware
  changes go through a `fold/*` topic branch and a fork-fold rebuild.
- Stale-checkout trap: fetch before assuming a submodule pointer builds.
  ryzen-shine's tree was 14 commits behind and did not compile until synced.
- Do not lower the host read timeout to "fix" a stall; a migrating keyboard
  is not reading requests, and a fresh session's first write then dies on the
  kernel's 5 s interrupt-OUT timeout.
- `try_send` in `set_action` is not a fix: it silently drops persists.
- `moergo-control` ships no reboot command; `Reboot` exists over Rynk
  (`handlers/system.rs`) if you need one for the drift check.

## Addendum (2026-09-14, after a first sync attempt on this machine)

- The checkout is already at the pins: glove80-config `838298a`, moergo-rmk
  `f429fc2`, rynkbench `76e9dbe`. Skip the pull step.
- Local work was stashed, not lost: outer `stash@{0}` "wip before master
  sync: TailorKey bilateral trial", moergo-rmk `stash@{0}` "jay-lenovo WIP
  before persist-stall sync: position-combo import refactor + control debug
  probes (superseded by f429fc2)". Leave both alone unless you need them.
- A `moergo-control` built outside the nix shell fails to load
  `libdbus-1.so.3`; build and run it inside `nix develop
  ./dependencies/moergo-rmk` (or use `bin/moergo-control`, which wraps that).
- Another Paseo agent on this daemon, `b72830e` "Restore Glove80 config via
  small flash-safe batches" (idle), did the earlier restore with a 120 s
  timeout and 4-key pages; `paseo logs b72830e` has its observations and
  the exact stall points. Do not run two agents against the keyboard at once.
