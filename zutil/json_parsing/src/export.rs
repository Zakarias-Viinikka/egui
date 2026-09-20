use serde_json::json;

use crate::new_category::NewCategory;
use crate::new_template::NewTemplate;
use crate::new_text::NewText;

pub fn build_export_json(
    categories: Vec<NewCategory>,
    texts: Vec<NewText>,
    templates: Vec<NewTemplate>,
) -> Result<String, serde_json::Error> {
    let mut blocks: Vec<serde_json::Value> = Vec::new();

    if !categories.is_empty() {
        blocks.push(json!({
            "parsing_instruct": "new_categories",
            "items": categories,
        }));
    }
    if !texts.is_empty() {
        blocks.push(json!({
            "parsing_instruct": "new_texts",
            "items": texts,
        }));
    }
    if !templates.is_empty() {
        blocks.push(json!({
            "parsing_instruct": "new_templates",
            "items": templates,
        }));
    }

    serde_json::to_string_pretty(&blocks)
}
