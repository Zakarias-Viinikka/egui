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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffRow {
    pub left: Option<Side>,
    pub right: Option<Side>,
    pub kind: RowKind,
}
