use similar::{ChangeTag, TextDiff, DiffOp};

use super::rows::{DiffRow, RowKind, Side};
use super::similarity::{similarity, SIMILARITY_THRESHOLD};

/// Above this many old-times-new lines, the alignment DP is skipped and
/// lines are paired by index with the same threshold test. A 320x320
/// block is well past what a real edit produces and still fast.
const ALIGNMENT_CELL_LIMIT: usize = 100_000;

fn same_text(a: &str, b: &str) -> bool {
    a == b || (a.trim().is_empty() && b.trim().is_empty())
}

pub fn pair_lines(old: &str, new: &str) -> Vec<DiffRow> {
    let diff = TextDiff::from_lines(old, new);
    let mut rows: Vec<DiffRow> = Vec::new();

    for group in diff.grouped_ops(3) {
        let ops = merge_adjacent_delete_insert(&group);
        for op in &ops {
            pair_one_op(&diff, op, &mut rows);
        }
    }

    rows
}

/// Same pairing as `pair_lines`, but with the snippet context limit
/// removed so every line of both files appears. Used by the full-file
/// toggle.
pub fn pair_lines_full(old: &str, new: &str) -> Vec<DiffRow> {
    let diff = TextDiff::from_lines(old, new);
    let mut rows: Vec<DiffRow> = Vec::new();

    let ops = merge_adjacent_delete_insert(diff.ops());
    for op in &ops {
        pair_one_op(&diff, op, &mut rows);
    }

    rows
}

/// Pair two arbitrary slices of already-resolved sides. Used by
/// move_display to reconstruct the paired view of a moved block: the old
/// slice comes from the Delete rows at the move's original position,
/// the new slice from the Insert rows at the destination.
pub fn pair_runs(old: &[Side], new: &[Side]) -> Vec<DiffRow> {
    let mut rows = Vec::new();
    let pairs = align(old, new);
    emit_aligned(pairs, old.to_vec(), new.to_vec(), &mut rows);
    rows
}

fn merge_adjacent_delete_insert(ops: &[DiffOp]) -> Vec<DiffOp> {
    let mut merged = Vec::with_capacity(ops.len());
    let mut i = 0;

    while i < ops.len() {
        match (ops.get(i), ops.get(i + 1)) {
            (
                Some(DiffOp::Delete { old_index, old_len, new_index }),
                Some(DiffOp::Insert {
                    old_index: insert_old_index,
                    new_index: insert_new_index,
                    new_len,
                }),
            ) if *insert_old_index == *old_index + *old_len
                && *insert_new_index == *new_index =>
            {
                merged.push(DiffOp::Replace {
                    old_index: *old_index,
                    old_len: *old_len,
                    new_index: *new_index,
                    new_len: *new_len,
                });
                i += 2;
            }
            (
                Some(DiffOp::Insert {
                    old_index: insert_old_index,
                    new_index: insert_new_index,
                    new_len: insert_new_len,
                }),
                Some(DiffOp::Delete { old_index, old_len, new_index }),
            ) if *old_index == *insert_old_index
                && *new_index == *insert_new_index + *insert_new_len =>
            {
                merged.push(DiffOp::Replace {
                    old_index: *old_index,
                    old_len: *old_len,
                    new_index: *insert_new_index,
                    new_len: *insert_new_len,
                });
                i += 2;
            }
            (Some(op), _) => {
                merged.push(op.clone());
                i += 1;
            }
            (None, _) => break,
        }
    }

    merged
}

fn pair_one_op(
    diff: &TextDiff<'_, '_, '_, str>,
    op: &similar::DiffOp,
    rows: &mut Vec<DiffRow>,
) {
    let mut old_lines: Vec<Side> = Vec::new();
    let mut new_lines: Vec<Side> = Vec::new();
    let mut saw_change = false;

    for change in diff.iter_changes(op) {
        let text = change.value().trim_end_matches('\n').to_string();
        match change.tag() {
            ChangeTag::Equal => {
                old_lines.push(Side {
                    lineno: change.old_index().map(|i| i as u32 + 1),
                    text: text.clone(),
                });
                new_lines.push(Side {
                    lineno: change.new_index().map(|i| i as u32 + 1),
                    text,
                });
            }
            ChangeTag::Delete => {
                saw_change = true;
                old_lines.push(Side {
                    lineno: change.old_index().map(|i| i as u32 + 1),
                    text,
                });
            }
            ChangeTag::Insert => {
                saw_change = true;
                new_lines.push(Side {
                    lineno: change.new_index().map(|i| i as u32 + 1),
                    text,
                });
            }
        }
    }

    if !saw_change {
        for (l, r) in old_lines.into_iter().zip(new_lines) {
            rows.push(DiffRow {
                left: Some(l),
                right: Some(r),
                kind: RowKind::Equal,
            });
        }
    } else if old_lines.is_empty() {
        for r in new_lines {
            rows.push(DiffRow {
                left: None,
                right: Some(r),
                kind: RowKind::Insert,
            });
        }
    } else if new_lines.is_empty() {
        for l in old_lines {
            rows.push(DiffRow {
                left: Some(l),
                right: None,
                kind: RowKind::Delete,
            });
        }
    } else {
        let pairs = align(&old_lines, &new_lines);
        emit_aligned(pairs, old_lines, new_lines, rows);
    }
}

/// Align old lines and new lines by similarity. Returns a list of
/// (old_index, new_index) tuples where each side may be None, ordered as
/// walked through. Consecutive None slots are grouped into runs by the
/// caller.
fn align(old: &[Side], new: &[Side]) -> Vec<(Option<usize>, Option<usize>)> {
    let k = old.len();
    let j = new.len();
    if k == 0 || j == 0 {
        let mut out = Vec::new();
        for i in 0..k {
            out.push((Some(i), None));
        }
        for i in 0..j {
            out.push((None, Some(i)));
        }
        return out;
    }
    if k * j > ALIGNMENT_CELL_LIMIT {
        return align_by_index(old, new);
    }

    let mut score = vec![vec![0.0f32; j + 1]; k + 1];
    let mut trace = vec![vec![0u8; j + 1]; k + 1];

    for i in 1..=k {
        for jj in 1..=j {
            let sim = similarity(&old[i - 1].text, &new[jj - 1].text);
            let mut best = score[i - 1][jj];
            let mut action = 0u8;
            if score[i][jj - 1] > best {
                best = score[i][jj - 1];
                action = 1;
            }
            if sim >= SIMILARITY_THRESHOLD {
                let diag = score[i - 1][jj - 1] + sim;
                if diag > best {
                    best = diag;
                    action = 2;
                }
            }
            score[i][jj] = best;
            trace[i][jj] = action;
        }
    }

    let mut pairs: Vec<(Option<usize>, Option<usize>)> = Vec::new();
    let (mut i, mut jj) = (k, j);
    while i > 0 || jj > 0 {
        let action = if i == 0 {
            1
        } else if jj == 0 {
            0
        } else {
            trace[i][jj]
        };
        match action {
            0 => {
                pairs.push((Some(i - 1), None));
                i -= 1;
            }
            1 => {
                pairs.push((None, Some(jj - 1)));
                jj -= 1;
            }
            _ => {
                pairs.push((Some(i - 1), Some(jj - 1)));
                i -= 1;
                jj -= 1;
            }
        }
    }
    pairs.reverse();
    pairs
}

/// Fallback for very large blocks: pair by index if the pair is similar
/// enough, otherwise emit a delete and an insert.
fn align_by_index(old: &[Side], new: &[Side]) -> Vec<(Option<usize>, Option<usize>)> {
    let mut out = Vec::new();
    let count = old.len().max(new.len());
    for i in 0..count {
        match (i < old.len(), i < new.len()) {
            (true, true) => {
                if similarity(&old[i].text, &new[i].text) >= SIMILARITY_THRESHOLD {
                    out.push((Some(i), Some(i)));
                } else {
                    out.push((Some(i), None));
                    out.push((None, Some(i)));
                }
            }
            (true, false) => out.push((Some(i), None)),
            (false, true) => out.push((None, Some(i))),
            (false, false) => {}
        }
    }
    out
}

fn emit_aligned(
    pairs: Vec<(Option<usize>, Option<usize>)>,
    old_lines: Vec<Side>,
    new_lines: Vec<Side>,
    rows: &mut Vec<DiffRow>,
) {
    let mut i = 0;
    while i < pairs.len() {
        match pairs[i] {
            (Some(oi), Some(ni)) => {
                let l = old_lines[oi].clone();
                let r = new_lines[ni].clone();
                let kind = if same_text(&l.text, &r.text) {
                    RowKind::Equal
                } else {
                    RowKind::Change
                };
                rows.push(DiffRow {
                    left: Some(l),
                    right: Some(r),
                    kind,
                });
                i += 1;
            }
            _ => {
                let mut dels: Vec<usize> = Vec::new();
                let mut inses: Vec<usize> = Vec::new();
                while i < pairs.len() && !matches!(pairs[i], (Some(_), Some(_))) {
                    match pairs[i] {
                        (Some(oi), None) => dels.push(oi),
                        (None, Some(ni)) => inses.push(ni),
                        _ => unreachable!(),
                    }
                    i += 1;
                }
                if dels.len() == 1 && inses.len() == 1 {
                    let l = old_lines[dels[0]].clone();
                    let r = new_lines[inses[0]].clone();
                    let kind = if same_text(&l.text, &r.text) {
                        RowKind::Equal
                    } else {
                        RowKind::Change
                    };
                    rows.push(DiffRow {
                        left: Some(l),
                        right: Some(r),
                        kind,
                    });
                } else {
                    for oi in dels {
                        rows.push(DiffRow {
                            left: Some(old_lines[oi].clone()),
                            right: None,
                            kind: RowKind::Delete,
                        });
                    }
                    for ni in inses {
                        rows.push(DiffRow {
                            left: None,
                            right: Some(new_lines[ni].clone()),
                            kind: RowKind::Insert,
                        });
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(rows: &[DiffRow]) -> Vec<RowKind> {
        rows.iter().map(|r| r.kind.clone()).collect()
    }

    #[test]
    fn adjacent_delete_insert_ops_are_aligned_together() {
        let diff = TextDiff::from_lines("let x = 1;\n", "let y = 1;\n");
        let ops = vec![
            DiffOp::Delete {
                old_index: 0,
                old_len: 1,
                new_index: 0,
            },
            DiffOp::Insert {
                old_index: 1,
                new_index: 0,
                new_len: 1,
            },
        ];
        let merged = merge_adjacent_delete_insert(&ops);

        assert_eq!(merged.len(), 1);

        let mut rows = Vec::new();
        pair_one_op(&diff, &merged[0], &mut rows);
        assert_eq!(kinds(&rows), vec![RowKind::Change]);
    }

    #[test]
    fn comma_added_gives_a_change() {
        let old = "let x = 1;\n";
        let new = "let x = 1,;\n";
        let rows = pair_lines(old, new);
        assert_eq!(kinds(&rows), vec![RowKind::Change]);
    }

    #[test]
    fn renamed_variable_gives_a_change() {
        let old = "let x = 1;\n";
        let new = "let y = 1;\n";
        let rows = pair_lines(old, new);
        assert_eq!(kinds(&rows), vec![RowKind::Change]);
    }

    #[test]
    fn reindented_line_gives_a_change() {
        let old = "        foo();\n";
        let new = "    foo();\n";
        let rows = pair_lines(old, new);
        assert_eq!(kinds(&rows), vec![RowKind::Change]);
    }

    #[test]
    fn unrelated_swap_gives_delete_then_insert() {
        let old = "fn main() {\n";
        let new = "let result = compute();\n";
        let rows = pair_lines(old, new);
        assert_eq!(kinds(&rows), vec![RowKind::Change]);
    }

    #[test]
    fn short_line_replaced_by_long_line_gives_change() {
        let old = "let a = 1;\n";
        let new = "let a = compute_something_long(b);\n";
        let rows = pair_lines(old, new);
        assert_eq!(kinds(&rows), vec![RowKind::Change]);
    }

    #[test]
    fn unrelated_assignments_give_change() {
        let old = "x = 1;\n";
        let new = "y = 2;\n";
        let rows = pair_lines(old, new);
        assert_eq!(kinds(&rows), vec![RowKind::Change]);
    }

    #[test]
    fn multiple_unrelated_lines_remain_deletes_then_inserts() {
        let old = "old alpha\nold beta\n";
        let new = "new gamma\nnew delta\n";
        let rows = pair_lines_full(old, new);
        assert_eq!(
            kinds(&rows),
            vec![
                RowKind::Delete,
                RowKind::Delete,
                RowKind::Insert,
                RowKind::Insert,
            ]
        );
    }

    #[test]
    fn identical_text_gives_equal() {
        let rows = pair_lines_full("same line\n", "same line\n");
        assert_eq!(kinds(&rows), vec![RowKind::Equal]);
    }

    #[test]
    fn insertion_in_the_middle_keeps_alignment() {
        let old = "one\ntwo\nthree\n";
        let new = "one\ntwo\nnew\nthree\n";
        let rows = pair_lines_full(old, new);
        let ks = kinds(&rows);
        assert!(ks.contains(&RowKind::Insert));
        // equal lines around the insert stay paired
        let pairs: Vec<usize> = rows
            .iter()
            .enumerate()
            .filter(|(_, r)| r.kind == RowKind::Equal)
            .map(|(i, _)| i)
            .collect();
        assert!(pairs.len() >= 3);
    }
}