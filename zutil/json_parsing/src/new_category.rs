use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct NewCategory {
    pub name: String,
    #[serde(default = "default_type")]
    pub type_of_category: String,
}

fn default_type() -> String {
    "normal".to_string()
}
