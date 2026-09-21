use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct NewText {
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub meta_category: Option<String>,
    #[serde(default)]
    pub type_of_text: Option<String>,
    /// If true, clicking this text in a circle nav copies it to the clipboard
    /// instead of opening the view overlay. Defaults to false (view).
    #[serde(default)]
    pub copy_instead_of_view: Option<bool>,
}
