#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verb {
    Create,
    Replace,
    Overwrite,
    Remove,
    Move,
    Mkdir,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    pub verb: Verb,
    /// The path as written in the chain. May be absolute, start with ~/,
    /// or be relative. Resolution happens in play::play_forward.
    pub path: String,
    /// Move only: the destination path as written. None otherwise.
    pub dest: Option<String>,
    /// replace only: the text to find. Empty for create and overwrite.
    pub old_text: String,
    /// create, overwrite, replace: the text to put in place.
    pub new_text: String,
    /// replace only: how many occurrences to replace. None means one.
    pub count: Option<usize>,
}
