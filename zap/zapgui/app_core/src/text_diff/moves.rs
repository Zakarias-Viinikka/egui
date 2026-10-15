//! Move detection: find runs of deleted lines that reappeared as runs of
//! inserted lines, and mark them as a move rather than a delete + insert.
//!
//! Spans are in file line numbers, not row positions, so they apply to
//! the snippet view and the full-file view alike. Detection runs across
//! every step of a chain at once, so a move between two files is found
//! too.

use super::pair::pair_lines_full;
use super::rows::{DiffRow, RowKind};
use super::similarity::{dice_over_tokens, similarity};

/// A run smaller than this in line count or non-whitespace characters is
/// not worth calling a move.
const MIN_RUN_LINES: usize = 3;
const MIN_RUN_CHARS: usize = 60;

/// Cheap pre-check: Dice over tokens must reach this before the aligner
/// runs.
const DICE_FLOOR: f32 = 0.6;

/// After alignment, the matched-weight-over-longer-run score must reach
/// this.
const ALIGN_FLOOR: f32 = 0.7;

/// If either run exceeds this many lines, skip alignment and require a
/// higher Dice score.
const LONG_RUN: usize = 300;
const DICE_FLOOR_LONG: f32 = 0.8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveRole {
    From,
    To,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MoveSpan {
    pub role: MoveRole,
    pub first_line: u32,
    pub last_line: u32,
    pub label: String,
    pub edited: bool,
}

/// A candidate run: a maximal set of consecutive Delete-only rows (or
/// Insert-only rows) with their file line numbers and joined text.
struct Run {
    /// Step index this run belongs to.
    step: usize,
    role: MoveRole,
    first_line: u32,
    last_line: u32,
    /// Positions in the complete diff output.
    first_row: usize,
    last_row: usize,
    lines: Vec<String>,
    chars: usize,
}

impl Run {
    fn text(&self) -> String {
        self.lines.join("\n")
    }
}

/// Input: per-step rows (the flat `pair_lines` output for each step's
/// before/after), plus the display path of each step. Output: per-step
/// move spans, in the same order as the input.
pub fn detect(steps: &[(Vec<DiffRow>, String)]) -> Vec<Vec<MoveSpan>> {
    let mut runs_by_step: Vec<Vec<Run>> = Vec::with_capacity(steps.len());

    for (step_idx, (rows, _path)) in steps.iter().enumerate() {
        let mut runs = collect_runs(rows, step_idx);
        // Filter by size before they enter matching.
        runs.retain(|r| r.lines.len() >= MIN_RUN_LINES && r.chars >= MIN_RUN_CHARS);
        runs_by_step.push(runs);
    }

    let mut from_runs: Vec<&Run> = Vec::new();
    let mut to_runs: Vec<&Run> = Vec::new();
    for runs in &runs_by_step {
        for r in runs {
            match r.role {
                MoveRole::From => from_runs.push(r),
                MoveRole::To => to_runs.push(r),
            }
        }
    }

    // Cheap score, then confirm with alignment. Pairs sorted by score
    // descending, taken greedily, each run used once.
    let mut candidates: Vec<(f32, usize, usize)> = Vec::new();
    for (fi, f) in from_runs.iter().enumerate() {
        for (ti, t) in to_runs.iter().enumerate() {
            // A delete run and an insert run in the same step that touch
            // directly is a replacement, not a move.
            if f.step == t.step && runs_touch(f, t) {
                continue;
            }
            let d = dice_over_tokens(&f.text(), &t.text());
            if d < DICE_FLOOR {
                continue;
            }
            candidates.push((d, fi, ti));
        }
    }
    candidates.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    let mut from_used = vec![false; from_runs.len()];
    let mut to_used = vec![false; to_runs.len()];
    let mut matches: Vec<(&Run, &Run, f32)> = Vec::new();

    for (_score, fi, ti) in candidates {
        if from_used[fi] || to_used[ti] {
            continue;
        }
        let f = from_runs[fi];
        let t = to_runs[ti];

        let long = f.lines.len() > LONG_RUN || t.lines.len() > LONG_RUN;
        let confirmed = if long {
            dice_over_tokens(&f.text(), &t.text()) >= DICE_FLOOR_LONG
        } else {
            let s = similarity(&f.text(), &t.text());
            s >= ALIGN_FLOOR
        };
        if !confirmed {
            continue;
        }

        from_used[fi] = true;
        to_used[ti] = true;
        matches.push((f, t, similarity(&f.text(), &t.text())));
    }

    let mut out: Vec<Vec<MoveSpan>> = (0..steps.len()).map(|_| Vec::new()).collect();

    for (f, t, _score) in matches {
        let edited = !lines_identical(&f.lines, &t.lines);

        let to_path = &steps[t.step].1;
        let from_path = &steps[f.step].1;

        let from_label = if f.step == t.step {
            format!("moved to line {}", t.first_line)
        } else {
            format!("moved to {}:{}", to_path, t.first_line)
        };
        let from_label = if edited {
            format!("{from_label} (edited)")
        } else {
            from_label
        };

        let to_label = if f.step == t.step {
            format!("moved from line {}", f.first_line)
        } else {
            format!("moved from {}:{}", from_path, f.first_line)
        };
        let to_label = if edited {
            format!("{to_label} (edited)")
        } else {
            to_label
        };

        out[f.step].push(MoveSpan {
            role: MoveRole::From,
            first_line: f.first_line,
            last_line: f.last_line,
            label: from_label,
            edited,
        });
        out[t.step].push(MoveSpan {
            role: MoveRole::To,
            first_line: t.first_line,
            last_line: t.last_line,
            label: to_label,
            edited,
        });
    }

    // Merge spans that share an endpoint (same role, same range) into one
    // label, so a run discovered twice shows one strip.
    for spans in out.iter_mut() {
        spans.sort_by_key(|s| (s.first_line, s.last_line));
        spans.dedup_by(|a, b| a.role == b.role && a.first_line == b.first_line && a.last_line == b.last_line);
    }

    out
}

fn runs_touch(f: &Run, t: &Run) -> bool {
    f.last_row + 1 == t.first_row || t.last_row + 1 == f.first_row
}

/// Build runs of consecutive Delete-only (or Insert-only) rows from a flat
/// row list. Rows with both sides populated (Change or Equal) end a run.
fn collect_runs(rows: &[DiffRow], step: usize) -> Vec<Run> {
    let mut out: Vec<Run> = Vec::new();
    let mut current: Option<Run> = None;

    for (row_idx, row) in rows.iter().enumerate() {
        let role = match row.kind {
            RowKind::Delete if row.left.is_some() && row.right.is_none() => {
                Some(MoveRole::From)
            }
            RowKind::Insert if row.right.is_some() && row.left.is_none() => {
                Some(MoveRole::To)
            }
            _ => None,
        };

        match role {
            Some(r) => {
                let (lineno, text) = match r {
                    MoveRole::From => (
                        row.left.as_ref().and_then(|s| s.lineno),
                        row.left.as_ref().map(|s| s.text.clone()).unwrap_or_default(),
                    ),
                    MoveRole::To => (
                        row.right.as_ref().and_then(|s| s.lineno),
                        row.right.as_ref().map(|s| s.text.clone()).unwrap_or_default(),
                    ),
                };
                let Some(lineno) = lineno else {
                    flush(&mut current, &mut out);
                    continue;
                };

                let chars = text.chars().filter(|c| !c.is_whitespace()).count();

                match &mut current {
                    Some(cur)
                        if cur.role == r
                            && cur.last_line + 1 == lineno
                            && cur.last_row + 1 == row_idx =>
                    {
                        cur.last_line = lineno;
                        cur.last_row = row_idx;
                        cur.lines.push(text);
                        cur.chars += chars;
                    }
                    _ => {
                        flush(&mut current, &mut out);
                        current = Some(Run {
                            step,
                            role: r,
                            first_line: lineno,
                            last_line: lineno,
                            first_row: row_idx,
                            last_row: row_idx,
                            lines: vec![text],
                            chars,
                        });
                    }
                }
            }
            None => flush(&mut current, &mut out),
        }
    }

    flush(&mut current, &mut out);
    out
}

fn flush(current: &mut Option<Run>, out: &mut Vec<Run>) {
    if let Some(run) = current.take() {
        out.push(run);
    }
}

fn lines_identical(a: &[String], b: &[String]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b.iter()).all(|(x, y)| x.trim() == y.trim())
}

/// Convenience: run detection on one step's before/after. Used by tests
/// and any single-step caller.
pub fn detect_one(before: &str, after: &str) -> Vec<MoveSpan> {
    let rows = pair_lines_full(before, after);
    let steps = vec![(rows, String::new())];
    detect(&steps).into_iter().next().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inserting_lines_above_a_block_is_not_a_move() {
        // Ten lines, then some filler, then the same ten lines.
        let mut old = String::new();
        let mut new = String::new();
        let block = "alpha = 1\nbeta = 2\ngamma = 3\ndelta = 4\nepsilon = 5\n";
        for _ in 0..4 {
            old.push_str("filler line\n");
        }
        old.push_str(block);
        for _ in 0..4 {
            new.push_str("filler line\n");
        }
        for _ in 0..4 {
            new.push_str("new stuff here\n");
        }
        new.push_str(block);

        let spans = detect_one(&old, &new);
        assert!(spans.is_empty(), "a natural shift is not a move: {spans:?}");
    }

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
        .join("\n") + "\n"
    }

    #[test]
    fn a_block_reordered_within_a_file_is_detected() {
        let prefix = stable_lines("prefix");
        let suffix = stable_lines("suffix");
        let block = moved_block();
        let old = format!("{prefix}{block}{suffix}");
        let new = format!("{prefix}{suffix}{block}");

        let spans = detect_one(&old, &new);
        assert!(
            spans.iter().any(|s| s.role == MoveRole::From),
            "expected a From span, got {spans:?}"
        );
        assert!(
            spans.iter().any(|s| s.role == MoveRole::To),
            "expected a To span, got {spans:?}"
        );
    }

    #[test]
    fn an_edited_move_is_detected_and_marked_edited() {
        let prefix = stable_lines("prefix");
        let suffix = stable_lines("suffix");
        let block = moved_block();
        let edited_block = block.replace(
            "calculate_gamma_value()",
            "calculate_updated_gamma_value()",
        );
        let old = format!("{prefix}{block}{suffix}");
        let new = format!("{prefix}{suffix}{edited_block}");

        let spans = detect_one(&old, &new);
        let from = spans
            .iter()
            .find(|s| s.role == MoveRole::From)
            .unwrap_or_else(|| panic!("expected a From span, got {spans:?}"));
        let to = spans
            .iter()
            .find(|s| s.role == MoveRole::To)
            .unwrap_or_else(|| panic!("expected a To span, got {spans:?}"));

        assert!(from.edited, "expected edited=true, got {from:?}");
        assert!(to.edited, "expected edited=true, got {to:?}");
        assert!(from.label.contains("(edited)"), "got {:?}", from.label);
        assert!(to.label.contains("(edited)"), "got {:?}", to.label);
    }

    #[test]
    fn small_runs_are_ignored() {
        let old = "one\n";
        let new = "\none\n";
        let spans = detect_one(old, new);
        assert!(spans.is_empty(), "got {spans:?}");
    }

    #[test]
    fn inserting_lines_above_a_block_with_an_edit_is_not_a_move() {
        let old = "line a here\nline b here\nline c here\nline d here\nline e here\n";
        let mut new = String::new();
        for _ in 0..4 {
            new.push_str("pad\n");
        }
        new.push_str("line a here\nline b here\nline c CHANGED here\nline d here\nline e here\n");
        let spans = detect_one(old, new);
        assert!(spans.is_empty(), "a shifted block is not a move: {spans:?}");
    }
}