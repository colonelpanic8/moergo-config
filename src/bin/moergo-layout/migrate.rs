//! Conservative physical migration between the two runtime matrix layouts.
use anyhow::{Context, Result, bail};
use toml::Value;

use crate::{address::Board, model};

fn forward(row: usize, col: usize) -> Option<(usize, usize)> {
    if col == 6 || col == 7 {
        match row {
            0..=2 => Some((4, if col == 6 { 4 - row } else { 11 - row })),
            3..=5 => Some((row - 3, col)),
            _ => None,
        }
    } else if (1..=4).contains(&row) && col < 14 {
        Some((row - 1, col))
    } else {
        None
    }
}

fn position(row: usize, col: usize, source: Board) -> Option<(usize, usize)> {
    if source == Board::Glove80 {
        forward(row, col)
    } else {
        (0..6)
            .flat_map(|r| (0..14).map(move |c| (r, c)))
            .find(|&(r, c)| forward(r, c) == Some((row, col)))
    }
}

fn coordinate(value: &Value, source: Board) -> Result<Option<Value>> {
    let pair = value.as_array().context("expected [row, col]")?;
    if pair.len() != 2 {
        bail!("expected exactly two coordinates");
    }
    let row = pair[0]
        .as_integer()
        .filter(|n| *n >= 0)
        .context("invalid row")? as usize;
    let col = pair[1]
        .as_integer()
        .filter(|n| *n >= 0)
        .context("invalid column")? as usize;
    if row >= source.rows() || col >= 14 {
        bail!("coordinate [{row}, {col}] outside source matrix");
    }
    Ok(position(row, col, source)
        .map(|(r, c)| Value::Array(vec![Value::Integer(r as i64), Value::Integer(c as i64)])))
}

// Return false to remove an entire selector or combo, never a subset of a combo.
fn remap(value: &mut Value, source: Board, path: &str, notes: &mut Vec<String>) -> Result<bool> {
    match value {
        Value::Table(table) => {
            if table.contains_key("led")
                || table.contains_key("zone")
                || table.get("key").is_some_and(Value::is_integer)
            {
                notes.push(format!("{path}: omitted board-specific selector {table:?}"));
                return Ok(false);
            }
            if let Some(key) = table.get_mut("key").filter(|v| {
                v.as_array()
                    .is_some_and(|a| a.first().is_some_and(Value::is_integer))
            }) {
                match coordinate(key, source)? {
                    Some(mapped) => *key = mapped,
                    None => {
                        notes.push(format!("{path}: omitted unmapped key {key}"));
                        return Ok(false);
                    }
                }
            }
            for name in ["positions", "hold_trigger_key_positions"] {
                if name == "positions" && !table.contains_key("output") {
                    continue;
                }
                if let Some(Value::Array(positions)) = table.get_mut(name) {
                    let mut mapped = Vec::new();
                    for p in positions.iter() {
                        if let Some(p) = coordinate(p, source)? {
                            mapped.push(p);
                        } else {
                            notes.push(format!("{path}.{name}: unmapped position {p}"));
                            if name == "positions" {
                                return Ok(false);
                            }
                        }
                    }
                    // Empty hold-trigger lists have different semantics; require manual repair.
                    if !positions.is_empty() && mapped.is_empty() {
                        bail!(
                            "{path}.{name}: no positions survive; edit the source policy before migrating"
                        );
                    }
                    *positions = mapped;
                }
            }
            for (name, child) in table.iter_mut() {
                if !(name == "key"
                    && child
                        .as_array()
                        .is_some_and(|a| a.first().is_some_and(Value::is_integer)))
                    && name != "positions"
                    && name != "hold_trigger_key_positions"
                    && !remap(child, source, &format!("{path}.{name}"), notes)?
                {
                    bail!("cannot safely omit {path}.{name}");
                }
            }
        }
        Value::Array(entries) => {
            let mut kept = Vec::new();
            for (i, mut entry) in std::mem::take(entries).into_iter().enumerate() {
                if remap(&mut entry, source, &format!("{path}[{i}]"), notes)? {
                    kept.push(entry);
                }
            }
            *entries = kept;
        }
        _ => {}
    }
    Ok(true)
}

pub fn migrate(text: &str, target: Board) -> Result<(String, Vec<String>)> {
    model::parse(text)?;
    let mut root: Value = toml::from_str(text)?;
    let rows = root.get("rows").and_then(Value::as_integer).unwrap_or(6);
    let source = match rows {
        6 => Board::Glove80,
        5 => Board::Go60,
        _ => bail!("unsupported source rows: {rows}"),
    };
    if source == target {
        bail!("source is already {}", target.name());
    }
    if root.get("cols").and_then(Value::as_integer).unwrap_or(14) != 14 {
        bail!("expected 14 columns");
    }
    let mut notes = vec![format!(
        "{} -> {}: finger rows aligned; lower Glove80 thumb arc maps to Go60 T1-T3, upper arc to bottom-row keys. Review thumb ergonomics.",
        source.name(),
        target.name()
    )];
    let layers = root
        .get_mut("layer")
        .and_then(Value::as_array_mut)
        .context("missing layers")?;
    for (index, layer) in layers.iter_mut().enumerate() {
        if let Some(keys) = layer.get_mut("keys") {
            let rows: Vec<_> = keys
                .as_str()
                .context("keys must be a string")?
                .lines()
                .map(model::split_tokens)
                .filter(|r| !r.is_empty())
                .collect();
            if rows.is_empty() {
                continue;
            }
            if rows.len() != source.rows() || rows.iter().any(|r| r.len() != 14) {
                bail!("layer {index}: expected {}x14 grid", source.rows());
            }
            let mut grid = vec![vec!["KC_TRNS".to_string(); 14]; target.rows()];
            for (r, row) in grid.iter_mut().enumerate() {
                for (c, key) in row.iter_mut().enumerate() {
                    let hole = if target == Board::Go60 {
                        position(r, c, target).is_none()
                    } else {
                        (r == 0 || r == 5) && (c == 5 || c == 8)
                    };
                    if hole {
                        *key = "--".into();
                    }
                }
            }
            for (r, row) in rows.iter().enumerate() {
                for (c, token) in row.iter().enumerate() {
                    if let Some((dr, dc)) = position(r, c, source) {
                        grid[dr][dc] = token.0.clone();
                    } else if token.is_bound() {
                        notes.push(format!("layer {index} [{r},{c}]: dropped {}", token.0));
                    }
                }
            }
            *keys = Value::String(format!(
                "\n{}\n",
                grid.iter()
                    .map(|r| r.join(" "))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
    }
    root.as_table_mut()
        .unwrap()
        .insert("rows".into(), Value::Integer(target.rows() as i64));
    root.as_table_mut().unwrap().remove("bluetooth_name");
    notes.push("Bluetooth name omitted; set a destination name explicitly. New Glove80 keys are transparent.".into());
    if target == Board::Glove80 {
        if root.as_table_mut().unwrap().remove("pointing").is_some() {
            notes.push("Omitted Go60 pointing configuration.".into());
        }
        if let Some(behavior) = root.get_mut("behavior").and_then(Value::as_table_mut)
            && behavior.remove("auto_mouse_layers").is_some()
        {
            notes.push("Omitted Go60 auto-mouse policies.".into());
        }
    }
    remap(&mut root, source, "config", &mut notes)?;
    let output = toml::to_string_pretty(&root)?;
    for warning in crate::usage::analyze(&model::parse(&output)?).warnings {
        notes.push(warning);
    }
    notes.push("Review combo inputs and layer access after dropped bindings; validate with moergo-control before applying. TOML comments are not retained.".into());
    Ok((output, notes))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mapping_is_bijective_for_all_sixty_keys() {
        let mut mapped = std::collections::BTreeSet::new();
        for r in 0..6 {
            for c in 0..14 {
                if let Some(p) = forward(r, c) {
                    assert!(mapped.insert(p));
                    assert_eq!(position(p.0, p.1, Board::Go60), Some((r, c)));
                }
            }
        }
        assert_eq!(mapped.len(), 60);
        assert_eq!(forward(3, 1), Some((2, 1)));
        assert_eq!(forward(3, 6), Some((0, 6)));
    }
    #[test]
    fn real_configs_migrate_both_ways() {
        for (text, target) in [
            (include_str!("../../../config/glove80.toml"), Board::Go60),
            (include_str!("../../../config/go60.toml"), Board::Glove80),
        ] {
            let (output, notes) = migrate(text, target).unwrap();
            let parsed = model::parse(&output).unwrap();
            assert_eq!(Board::detect(&parsed), Some(target));
            assert!(!notes.is_empty());
            assert_eq!(
                parsed.layers.len(),
                model::parse(text).unwrap().layers.len()
            );
        }
    }
    #[test]
    fn sparse_bindings_and_hold_triggers_move_together() {
        let mut value: Value = toml::from_str(
            "[[layer]]\n[[layer.key]]\nkey = [3,6]\naction = 'MO(1)'\n[behavior.morse]\nhold_trigger_key_positions = [[3,6],[0,0]]"
        ).unwrap();
        remap(&mut value, Board::Glove80, "config", &mut Vec::new()).unwrap();
        assert_eq!(value["layer"][0]["key"][0]["key"][0].as_integer(), Some(0));
        assert_eq!(
            value["behavior"]["morse"]["hold_trigger_key_positions"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        let mut empty: Value = toml::from_str("hold_trigger_key_positions = [[0,0]]").unwrap();
        assert!(remap(&mut empty, Board::Glove80, "config", &mut Vec::new()).is_err());
    }

    #[test]
    fn drops_whole_combo_and_remaps_matrix_selectors() {
        let mut v: Value = toml::from_str("combo = [{positions = [[0,0],[3,1]], output = 'KC_ESC'}]\nscene = [{key = [3,1], color = '#ffffff'}, {led = 1}]").unwrap();
        let mut notes = Vec::new();
        remap(&mut v, Board::Glove80, "config", &mut notes).unwrap();
        assert!(v["combo"].as_array().unwrap().is_empty());
        assert_eq!(v["scene"].as_array().unwrap().len(), 1);
        assert_eq!(v["scene"][0]["key"][0].as_integer(), Some(2));
    }
}
