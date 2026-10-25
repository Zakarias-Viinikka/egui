//! Turn the raw pair_lines rows of a single step into the display form
//! the viewer expects when the step contains the To side of a same-step
//! move: the old position stays as dim-ghost Delete rows, the new
//! position becomes paired Change rows so both sides show.

use super::moves::{MoveRole, MoveSpan};
use super::pair::pair_runs;
use super::rows::{DiffRow, RowKind, Side};

pub fn apply(rows: Vec<DiffRow>, spans: &[MoveSpan]) -> Vec<DiffRow> {
    let mut rows = rows;
    let mut edits: Vec<(usize, usize, Vec<DiffRow>)> = Vec::new();

    for to_span in spans.iter().filter(|s| s.role == MoveRole::To) {
        let from_span = match spans
            .iter()
            .find(|s| s.role == MoveRole::From && s.pair == to_span.pair)
        {
            Some(s) => s,
            None => continue,
        };

        let old_sides: Vec<Side> = rows
            .iter()
            .filter_map(|r| {
                if !matches!(r.kind, RowKind::Delete) {
                    return None;
                }
                let l = r.left.as_ref()?;
                let n = l.lineno?;
                if n >= from_span.first_line && n <= from_span.last_line {
                    Some(l.clone())
                } else {
                    None
                }
            })
            .collect();
        if old_sides.is_empty() {
            continue;
        }

        let start = rows.iter().position(|r| {
            matches!(r.kind, RowKind::Insert)
                && r.right
                    .as_ref()
                    .and_then(|s| s.lineno)
                    .map_or(false, |n| n >= to_span.first_line && n <= to_span.last_line)
        });
        let Some(start) = start else { continue };
        let mut end = start;
        while end + 1 < rows.len() {
            let r = &rows[end + 1];
            let in_span = matches!(r.kind, RowKind::Insert)
                && r.right
                    .as_ref()
                    .and_then(|s| s.lineno)
                    .map_or(false, |n| n >= to_span.first_line && n <= to_span.last_line);
            if !in_span {
                break;
            }
            end += 1;
        }

        let new_sides: Vec<Side> = rows[start..=end]
            .iter()
            .filter_map(|r| r.right.clone())
            .collect();
        if new_sides.is_empty() {
            continue;
        }

        let mut produced = pair_runs(&old_sides, &new_sides);
        for r in &mut produced {
            if let Some(n) = r.right.as_ref().and_then(|s| s.lineno) {
                if n >= to_span.first_line && n <= to_span.last_line {
                    r.kind = RowKind::Change;
                }
            }
        }
        edits.push((start, end, produced));
    }

    edits.sort_by_key(|(s, _, _)| *s);
    for (start, end, produced) in edits.into_iter().rev() {
        rows.splice(start..=end, produced);
    }

    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text_diff::moves::{detect, detect_one};
    use crate::text_diff::pair::pair_lines_full;

    fn stable_lines(group: &str) -> String {
        (0..8)
            .map(|i| format!("let stable_{group}_{i} = calculate_{group}_{i}();\n"))
            .collect()
    }

    fn moved_block() -> String {
        [
            "let moved_alpha_value = calculate_alpha_value();",
            "let moved_beta_value = calculate_beta_value();",
            "let moved_gamma_value = calculate_gamma_value();",
            "let moved_delta_value = calculate_delta_value();",
            "let moved_epsilon_value = calculate_epsilon_value();",
        ]
        .join("\n")
            + "\n"
    }

    #[test]
    fn same_file_move_produces_change_plus_delete() {
        let prefix = stable_lines("prefix");
        let suffix = stable_lines("suffix");
        let block = moved_block();
        let old = format!("{prefix}{block}{suffix}");
        let new = format!("{prefix}{suffix}{block}");

        let rows = pair_lines_full(&old, &new);
        let spans = detect_one(&old, &new);
        assert!(!spans.is_empty(), "expected a move, got nothing");

        let out = apply(rows, &spans);

        let changes = out.iter().filter(|r| r.kind == RowKind::Change).count();
        let deletes = out.iter().filter(|r| r.kind == RowKind::Delete).count();
        assert_eq!(changes, 5, "expected five Change rows");
        assert_eq!(deletes, 5, "expected five dim-eligible Delete rows");
    }

    #[test]
    fn edited_move_keeps_differences_visible() {
        let prefix = stable_lines("prefix");
        let suffix = stable_lines("suffix");
        let block = moved_block();
        let edited_block = block.replace(
            "calculate_gamma_value()",
            "calculate_updated_gamma_value()",
        );
        let old = format!("{prefix}{block}{suffix}");
        let new = format!("{prefix}{suffix}{edited_block}");

        let rows = pair_lines_full(&old, &new);
        let spans = detect_one(&old, &new);

        let out = apply(rows, &spans);

        let changes: Vec<&DiffRow> = out
            .iter()
            .filter(|r| r.kind == RowKind::Change)
            .collect();
        assert_eq!(changes.len(), 5);
        let differs = changes.iter().any(|r| {
            let l = r.left.as_ref().map(|s| s.text.as_str()).unwrap_or("");
            let rr = r.right.as_ref().map(|s| s.text.as_str()).unwrap_or("");
            l != rr
        });
        assert!(differs, "expected at least one Change row with different texts");
    }

    #[test]
    fn cross_step_spans_leave_rows_unchanged() {
        let prefix = stable_lines("prefix");
        let suffix = stable_lines("suffix");
        let block = moved_block();

        let from_old = format!("{prefix}{block}{suffix}");
        let from_new = format!("{prefix}{suffix}");

        let to_old = format!("{prefix}{suffix}");
        let to_new = format!("{prefix}{block}{suffix}");

        let steps = vec![
            (pair_lines_full(&from_old, &from_new), "/a.rs".to_string()),
            (pair_lines_full(&to_old, &to_new), "/b.rs".to_string()),
        ];
        let spans = detect(&steps);

        let step0_rows = pair_lines_full(&from_old, &from_new);
        let out0 = apply(step0_rows.clone(), &spans[0]);
        assert_eq!(out0.len(), step0_rows.len());
        for (a, b) in out0.iter().zip(step0_rows.iter()) {
            assert_eq!(a.kind, b.kind);
            assert_eq!(
                a.left.as_ref().and_then(|s| s.lineno),
                b.left.as_ref().and_then(|s| s.lineno)
            );
            assert_eq!(
                a.right.as_ref().and_then(|s| s.lineno),
                b.right.as_ref().and_then(|s| s.lineno)
            );
        }

        let step1_rows = pair_lines_full(&to_old, &to_new);
        let out1 = apply(step1_rows.clone(), &spans[1]);
        assert_eq!(out1.len(), step1_rows.len());
    }
}
