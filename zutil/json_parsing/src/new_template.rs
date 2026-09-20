use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct NewTemplate {
    pub title: String,
    pub content: String,
    #[serde(default)]
    pub instructions: Option<String>,
    #[serde(default)]
    pub example: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub meta_category: Option<String>,
}
