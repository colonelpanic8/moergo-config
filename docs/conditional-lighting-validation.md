# Conditional lighting validation follow-up

## Jay Lenovo findings

The 2026-09-11 report at
`~/Projects/glove80-config/.worktrees/runtime-layer-conditions/TEST-layer-conditions.md`
on `jay-lenovo` qualified the layer-set feature on advanced firmware, including
true/false gates, ordered matching rules, readback, and restoration of the
original persistent configuration. Its published test branch did not build
because it pinned an older RMK host-client baseline. A scratch build worked
after baseline repairs.

The report also identified a real validation omission: advanced device
readback accepted contradictory singular/layer-set predicates and overlapping
active/inactive masks. Input TOML already rejected both. Capability display
omitted the flags used to negotiate advanced reads and writes.

## Published repair

The layer-set host changes now build on MoErgo `560b2759` and its already
qualified RMK `dd592a3d` baseline. RMK and embedded firmware are unchanged.
Advanced readback calls the same conditional validator as TOML input, and the
CLI prints the relevant runtime conditional capability flags. The browser
configuration codec uses advanced cells so layer sets survive import/export;
unrepresentable host-lock predicates fail explicitly instead of being lost.

Rynkbench checks changed lighting-table capacity and conditional predicate
support before any import writes. Syntax/validation failures stay local.
A request that stops answering still closes the session after five seconds;
a timeout is not proof of a keyboard reboot. Moosy's exact failing file and
session trace remain necessary to establish the reported disconnect's cause.

## Validation

- Focused native layer-condition, conditional-table, and rule-order tests pass,
  including contradictory advanced readback and older-firmware rejection.
- Strict all-target clippy and formatting pass for the modified host packages.
- Glove80 and Go60 runtime configurations and stock compiled-source parity pass.
- The generated browser configuration WASM builds; browser tests cover layer-set
  round trips, contradiction rejection, and zero writes after preflight errors.

No firmware was flashed or live runtime configuration changed in this follow-up.
The old experimental workspaces remain available with their original evidence.
