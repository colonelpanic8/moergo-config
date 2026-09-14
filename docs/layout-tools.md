# Offline layout tools (`moergo-layout`)

`moergo-layout` analyzes and transforms runtime configuration TOML files
(`config/glove80.toml`-style) without a connected keyboard. Results flow
through the normal runtime workflow — `moergo-control config diff/apply` —
so none of this requires a firmware flash.

Run it through the Justfile:

```sh
just usage                 # behavior usage report for all managed configs
just layout <subcommand>   # any moergo-layout invocation
```

## `usage` — behavior usage and orphan detection

```sh
just layout usage config/glove80.toml [--keycodes] [--check]
```

For each config the report shows:

- every layer with its bound-key count, lighting scene cell count, and the
  exact sites that activate it (`MO`/`LT`/`TG`/`TO`/`LMT` keys, morse holds,
  combo outputs), plus reachability from the default layer;
- every morse (`TD(n)`), macro (`MACRO(n)`), and morse profile with its
  reference count and sites;
- combo placement per layer and `USER(n)` hook usage;
- warnings for orphans (defined but never referenced), dangling references
  (referenced but not defined), unreachable layers, and scenes or combos
  targeting undefined layers.

`--check` exits non-zero when any warning fires, so it can gate CI.
`--keycodes` appends a frequency histogram of base keycodes.

## `os` — dual-OS modifier variants

```sh
just layout os mac config/glove80.toml -o config/glove80-mac.toml
```

Generates the other OS's variant of a config by swapping Ctrl and GUI in
every action binding: keymap grids, `[[layer.bind]]` overrides, morse
tap/hold actions, combo keys and outputs, forks, and macro operations.
Bare keycodes (`KC_LCTL`), wrapper calls (`LCTL(KC_C)`), `MOD()` masks
(`MOD_LCTL`), and typed-morse modifier names (`LCtrl`) are all covered.
The swap is its own inverse, so `os pc` on a mac-canonical file does the
same thing; the scheme argument only documents intent.

Switching OS is then a runtime operation:

```sh
./bin/moergo-control config apply config/glove80-mac.toml   # to macOS
./bin/moergo-control config apply config/glove80.toml       # back to PC
```

Comments and unrelated values survive generation, but treat variants as
build products: regenerate them after editing the canonical file instead
of editing them directly.

## Persistence: what `config apply` waits for

Every keymap cell, name, morse, combo, and lighting cell written over Rynk
becomes a new item in the firmware's flash store (RMK on top of
`sequential-storage`). The firmware queues each write on a bounded channel
(sixteen entries on these boards, `flash_channel_size`) and only accepts a
keymap page it can queue whole: it waits up to half a second for room and
otherwise answers `Busy` with nothing applied, so the Rynk session keeps
answering. A page longer than the channel streams through it instead and
blocks the session until it fits, which is why hosts keep pages short. When
the store's current page fills, the next write closes it and migrates the
previous page's live items through radio-scheduled flash timeslots, which
holds the queue for tens of seconds. On a nearly full store that happens
every few dozen writes.

`config apply` is built around that:

- Only cells that differ from what the keyboard holds are written, in pages of
  four (`MOERGO_PERSIST_BATCH` overrides; `MOERGO_WRITE_WHOLE_LAYERS=1`
  restores the old rewrite-every-cell behaviour).
- After every page it waits for the flash queue to drain with a layer
  metadata read, which the firmware serves behind everything queued before
  it. Waits of two seconds or more are printed as they happen, and a summary
  of cells written and time spent waiting follows the layer writes.
- A page the firmware answers `Busy` is sent again after 100 ms, for up to
  five minutes; the summary counts those retries.
- The USB transport tolerates five minutes of silence before declaring the
  link dead (`MOERGO_RYNK_READ_TIMEOUT_SECS`). Retrying sooner does not help:
  a keyboard mid-migration is not reading requests, so a fresh session's first
  write fails on the kernel's own interrupt timeout.

`./bin/moergo-control keymap persist-probe [--rounds N] [--writes]` measures
the queue drain time and, with `--writes`, one real item's persist time. A
single item taking seconds means the settings partition is nearly full.

The store used to fill fast because every rebuild re-stored all sixteen
compiled layers before any user data (about 22 KiB of items); eight 4 KiB
sectors left it over 90 % full. A fresh store now seeds only its config
records, since boot overlays stored cells on the compiled keymap anyway, and
the boards use twenty-four sectors at `0xdc000` (`crates/*/keyboard.toml`,
`[storage]`). Flashing a build with a different partition wipes the store, as
every rebuild already does, so apply the runtime config afterwards. See
`QUALIFICATION-persist-stalls.md` for the measurements behind all of this.

`./bin/moergo-control storage wipe [--yes]` is the factory reset: it erases
every persisted setting on the central (keymap, layer names, combos, morses,
macros, lighting, Bluetooth pairings) and reboots it on the compiled
defaults. The halves re-pair on their own; computers and phones must be
paired again. Follow it with `just apply` to restore the runtime config.

## `alpha` — alternate alpha layouts from a QWERTY source

```sh
just layout alpha colemak config/glove80.toml -o config/glove80-colemak.toml
just layout alpha colemak-dh config/tailorkey-v52-bilateral.toml --layers 0,1,2
just layout alpha dvorak config/go60.toml
```

Remaps the alpha (and, for Dvorak, punctuation) keys of a QWERTY config to
Colemak, Colemak-DH, or Dvorak. Only the default layer is touched unless
`--layers` lists more; leave positional layers (Games/WASD) unlisted. The
remap follows keycodes into wrappers, so `LT(1,KC_SCLN)` becomes
`LT(1,KC_O)` under Colemak and autoshift pairs like
`TH(KC_Q, LSFT(KC_Q), autoshift)` stay consistent.

For a TailorKey-style config, remap every layer that repeats alphas (Base,
Typing, AutoShift): `--layers 0,1,2`.

Like `os`, the output is applied with `moergo-control config apply`. An
alternative that needs no second file: keep an alternate-alpha layer in a
spare slot and flip between them with
`./bin/moergo-control keymap default <layer>` — the persistent default
layer is a runtime setting.

## `preset` — partial layout fragments

```sh
just layout preset show presets/hrm-bilateral.toml
just layout preset apply presets/symbols-layer.toml config/glove80.toml -o /tmp/merged.toml
```

A preset is a TOML file carrying a *portion* of a layout — home row mods,
a symbols layer, a system-controls layer — that `apply` merges into an
existing runtime config. The output goes through the same
`moergo-control config diff/apply` workflow as everything else.

A preset may contain:

- `[preset]` — required header: `name`, optional `description`, optional
  `geometry = [rows, cols]` checked against the target's default layer,
  optional `boards = ["glove80", "go60"]` naming the boards the preset
  supports (checked against the board detected from the target's grid:
  6x14 is a Glove80, 5x14 a Go60);
- `[[layer]]` — new layers, appended to the target's next free slots. Each
  needs an identifier-like `id`, which becomes its `$reference` name. A
  board-portable preset provides one grid per board via `keys.glove80` and
  `keys.go60` instead of a single `keys` string;
- `[[morse]]`, `[[macro]]`, `[[combo]]`, `[[fork]]` — appended behavior
  entries (`name` required; clashes with the target are errors);
- `[behavior.morse.profiles.*]` — named morse profiles. A profile that
  already exists identically is skipped; one that exists with different
  settings is an error;
- `[[lighting.scene]]` — per-key scene cells (the target must already have
  a `[lighting]` section for these to extend);
- `[[patch]]` — edits to *existing* layers: a `layer` selector plus either
  or both of a `keys` template grid and `[[patch.key]]` entries with an
  `at` position and an `action`. In a template grid, transparent cells
  (`_______`/`KC_TRNS`) and `--` leave the target's binding alone and
  every other cell overwrites it, with `$key` standing for the binding
  being replaced — so a grid of `TH($key, LSFT($key), autoshift)` wraps a
  whole block in place. Template grids accept the per-board
  `keys.glove80`/`keys.go60` form. Explicit `[[patch.key]]` entries apply
  after the template, so they can refine cells it covered.

### Physical section addresses

Anywhere a cell position appears — a patch's `at` or a scene cell's
`key` — three forms are accepted:

- `[row, col]` — raw matrix coordinates (board-specific);
- `"LH-C4R4"` — a MoErgo physical section address, resolved against the
  detected board. `LH`/`RH` name the half, finger columns count from the
  thumb side outward (`C1`–`C6`), rows from the top down, and thumb keys
  are `T1` onward (the Glove80 numbers its upper fan first);
- `{ glove80 = "LH-C4R4", go60 = "LH-C4R3" }` — a per-board table picking
  the entry for the detected board. This is how one preset targets the
  same physical key on both boards: the Go60 has no F-row, so its home
  row is `R3` where the Glove80's is `R4`.

The shipped `hrm-bilateral` and `symbols-layer` presets use the per-board
form and apply to either board; `magic-glove80` and `magic-go60` declare
`boards` restrictions and refuse the other board with a clear error.

Slots are resolved at apply time. Inside any action string, `$name`
refers to a fragment-defined layer id, morse name, or macro name and is
rewritten to the assigned index: `MO($symbols)`, `TD($sym_hold)`,
`MACRO($greeting)`. Plain numbers are absolute target slots. The `layer`
field of combos, scenes, and patches additionally accepts `"default"`
(the target's default layer) or a target layer id/name.

In a patch action, `$key` stands for whatever is bound at that cell, so
`MT($key, LGui, hrm_pinky)` wraps the existing key and works on any alpha
layout. `$key` requires a plain keycode at the cell — a cell already
holding a wrapped action fails loudly rather than silently nesting, and
physical holes (`--`) cannot be patched. An action without `$key` is a
deliberate overwrite.

`apply` prints the slot assignments and every patched cell to stderr,
then re-analyzes the merged config with the `usage` machinery and reports
any orphan/dangling/reachability warnings it introduced. Unknown preset
sections (for example `[lighting.background]`) are rejected rather than
merged, so a preset cannot silently take over global settings.

Shipped presets live in `presets/`:

- `hrm-bilateral.toml` — bilateral GACS home row mods: four per-finger
  TailorKey timing profiles, eight `$key`-wrapping patches on the home
  row, and dim per-finger scene cells. Applies to both boards;
- `autoshift.toml` — a template-grid patch wrapping the number and alpha
  block of the existing default layer in
  `TH($key, LSFT($key), autoshift)` with the TailorKey timing profile:
  tap types the key, hold types its shifted pair. Applies to both boards
  and any alpha layout, since each cell wraps whatever is bound there;
- `symbols-layer.toml` — a programmer symbols layer in the next free
  slot, entered by wrapping a thumb key in `LT($symbols, $key)` (the
  Glove80's home-row thumb, the Go60's upper-outer thumb), with the
  bracket cluster lit while active. Applies to both boards;
- `magic-glove80.toml` / `magic-go60.toml` — a Magic-style
  system-controls layer per board (BLE slot select, USB output, lighting
  controls, bootloader/reboot with the right-half forward) behind an
  `MO($magic)` key.

## Verification

Generated variants should always pass the authoritative schema check
before being applied:

```sh
./bin/moergo-control config validate <variant.toml>
```

The tool's own tests run with the rest of the host crate:

```sh
nix develop ./dependencies/moergo-rmk --command cargo test --bin moergo-layout
```
