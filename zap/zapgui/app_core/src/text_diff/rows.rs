use std::ops::Range;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RowKind {
    Equal,
    Change,
    Insert,
    Delete,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Side {
    pub lineno: Option<u32>,
    pub text: String,
    /// Byte ranges inside `text` that differ from the paired line.
    /// Filled only on Change rows.
    pub changed: Vec<Range<usize>>,
}

impl Side {
    pub fn new(lineno: Option<u32>, text: String) -> Self {
        Self {
            lineno,
            text,
            changed: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffRow {
    pub left: Option<Side>,
    pub right: Option<Side>,
    pub kind: RowKind,
    /// True on a Change row whose lines are similar enough to count as an
    /// edit (drawn blue). False everywhere else.
    pub edit: bool,
}

impl DiffRow {
    pub fn new(left: Option<Side>, right: Option<Side>, kind: RowKind) -> Self {
        Self {
            left,
            right,
            kind,
            edit: false,
        }
    }
}