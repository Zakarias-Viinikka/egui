use similar::{ChangeTag, TextDiff};

use super::rows::{DiffRow, RowKind, Side};

fn same_text(a: &str, b: &str) -> bool {
    a == b || (a.trim().is_empty() && b.trim().is_empty())
}

pub fn pair_lines(old: &str, new: &str) -> Vec<DiffRow> {
    let diff = TextDiff::from_lines(old, new);
    let mut rows: Vec<DiffRow> = Vec::new();

    for group in diff.grouped_ops(3) {
        for op in group {
            let mut old_lines: Vec<Side> = Vec::new();
            let mut new_lines: Vec<Side> = Vec::new();

            for change in diff.iter_changes(&op) {
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
                        old_lines.push(Side {
                            lineno: change.old_index().map(|i| i as u32 + 1),
                            text,
                        });
                    }
                    ChangeTag::Insert => {
                        new_lines.push(Side {
                            lineno: change.new_index().map(|i| i as u32 + 1),
                            text,
                        });
                    }
                }
            }

            if old_lines.len() == new_lines.len() {
                for (l, r) in old_lines.into_iter().zip(new_lines) {
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
                }
            } else {
                let count = old_lines.len().max(new_lines.len());
                let mut old_iter = old_lines.into_iter();
                let mut new_iter = new_lines.into_iter();
                for _ in 0..count {
                    let left = old_iter.next();
                    let right = new_iter.next();
                    let kind = match (&left, &right) {
                        (Some(l), Some(r)) => {
                            if same_text(&l.text, &r.text) {
                                RowKind::Equal
                            } else {
                                RowKind::Change
                            }
                        }
                        (Some(_), None) => RowKind::Delete,
                        _ => RowKind::Insert,
                    };
                    rows.push(DiffRow { left, right, kind });
                }
            }
        }
    }

    rows
}
