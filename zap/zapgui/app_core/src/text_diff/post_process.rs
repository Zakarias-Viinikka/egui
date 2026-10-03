use super::rows::{DiffRow, RowKind, Side};

pub struct DiffRowProcessedForUi {
    pub rows: Vec<DiffRow>,
    pub kind: RowKind,
}

fn run_kind(rows: &[DiffRow]) -> RowKind {
    let mut kind: Option<RowKind> = None;
    for r in rows.iter().filter(|r| r.kind != RowKind::Equal) {
        kind = match kind {
            None => Some(r.kind.clone()),
            Some(k) if k == r.kind => Some(k),
            Some(_) => Some(RowKind::Change),
        };
    }
    kind.unwrap_or(RowKind::Equal)
}

fn side_is_blank(side: &Option<Side>) -> bool {
    side.as_ref()
        .map(|s| s.text.trim().is_empty())
        .unwrap_or(true)
}

fn neutralize(r: &DiffRow) -> DiffRow {
    let mut row = r.clone();
    if row.kind != RowKind::Equal && side_is_blank(&row.left) && side_is_blank(&row.right) {
        row.kind = RowKind::Equal;
    }
    row
}

fn flush_equals(out: &mut Vec<DiffRowProcessedForUi>, equals: &mut Vec<DiffRow>) {
    if !equals.is_empty() {
        out.push(DiffRowProcessedForUi {
            rows: std::mem::take(equals),
            kind: RowKind::Equal,
        });
    }
}

pub fn process(rows: &[DiffRow]) -> Vec<DiffRowProcessedForUi> {
    let mut out: Vec<DiffRowProcessedForUi> = Vec::new();
    let mut equals: Vec<DiffRow> = Vec::new();
    let mut i = 0;

    while i < rows.len() {
        if rows[i].kind == RowKind::Equal {
            equals.push(rows[i].clone());
            i += 1;
            continue;
        }

        let start = i;
        while i < rows.len() && rows[i].kind != RowKind::Equal {
            i += 1;
        }

        let run: Vec<DiffRow> = rows[start..i].iter().map(neutralize).collect();
        if run.iter().all(|r| r.kind == RowKind::Equal) {
            equals.extend(run);
            continue;
        }

        let context = equals.pop();
        flush_equals(&mut out, &mut equals);

        let mut block: Vec<DiffRow> = Vec::new();
        block.extend(context);
        block.extend(run);
        let kind = run_kind(&block);
        out.push(DiffRowProcessedForUi { rows: block, kind });
    }

    flush_equals(&mut out, &mut equals);
    out
}

pub fn debug_format(blocks: &[DiffRowProcessedForUi]) -> String {
    let mut out = String::new();
    for (i, block) in blocks.iter().enumerate() {
        out.push_str(&format!("[{}] {:?}\n", i, block.kind));
        for row in &block.rows {
            let l = match &row.left {
                Some(s) => format!(
                    "  L {:>4} | {}",
                    s.lineno.map(|n| n.to_string()).unwrap_or_else(|| "?".into()),
                    s.text
                ),
                None => "  L    . |".to_string(),
            };
            let r = match &row.right {
                Some(s) => format!(
                    "  R {:>4} | {}",
                    s.lineno.map(|n| n.to_string()).unwrap_or_else(|| "?".into()),
                    s.text
                ),
                None => "  R    . |".to_string(),
            };
            out.push_str(&l);
            out.push('\n');
            out.push_str(&r);
            out.push('\n');
        }
        out.push('\n');
    }
    out
}
