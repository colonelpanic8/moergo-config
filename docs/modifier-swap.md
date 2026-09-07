# Runtime Control/GUI swapping

`CTRL_GUI_TOG` toggles left Control ↔ left GUI and right Control ↔ right GUI
across every layer. `CtrlGuiSwapToggle` and `CG_TOGG` are accepted aliases.
Both Glove80 and Go60 support the action with firmware containing the
`feat/ctrl-gui-swap` RMK topic.

The toggle fires on release. If a keyboard chord or one-shot modifier is active,
it waits until that chord ends. Toggling twice while waiting cancels the change.
The mode starts disabled after reboot; it is neither persistent nor tied to a
Bluetooth profile.

The swap covers ordinary modifiers, hold-taps, one-shots, modified keys, and
macro key actions. Shift and Alt stay unchanged. The keymap and fork matching
continue to use their original logical modifiers. A global swap also changes
literal Control shortcuts; it does not distinguish terminal shortcuts from
application shortcuts.

## Bind a key

In an existing `[[layer.key]]` entry in `config/glove80.toml` or
`config/go60.toml`, set:

```toml
action = "CTRL_GUI_TOG"
```

## Bind a combo

For example, add this at the top level of the runtime TOML:

```toml
[[combo]]
name = "Swap Control and GUI"
keys = ["KC_J", "KC_K"]
output = "CTRL_GUI_TOG"
```

Omitting `layer` allows the combo on every layer where those actions are present.
Use `positions` instead of `keys` for a combo tied to physical keys across layers.
Choose triggers that do not overlap an existing combo.

Validate both configs with `just check`. After installing firmware that supports
the action, run `just diff` then `just apply` for Glove80, or `just go60-diff`
then `just go60-apply` for Go60. Binding changes thereafter require only runtime
configuration, not another flash.

## Keycode compatibility

The runtime CLI uses `0x7C05` for this action and converts it to Rynk's typed
`KeyboardAction::CtrlGuiSwapToggle`. Its existing `0x701D` notation continues to
mean a held right-side modifier combination. RMK's Vial protocol uses QMK's
standard `CG_TOGG` value (`0x701D`); these are different host encodings.
