use serde::de::Error as _;
use serde::Deserialize;
use serde_json;

use crate::new_category::NewCategory;
use crate::new_template::NewTemplate;
use crate::new_text::NewText;

#[derive(Deserialize)]
struct RawBlock {
    #[serde(default)]
    parsing_instruct: Option<String>,
    items: serde_json::Value,
}

#[derive(Debug, Clone)]
pub enum ParsedPayload {
    NewTexts(Vec<NewText>),
    NewTemplates(Vec<NewTemplate>),
    NewCategories(Vec<NewCategory>),
}

pub fn parse_pasted_json(raw: &str) -> Result<Vec<ParsedPayload>, serde_json::Error> {
    let blocks: Vec<RawBlock> = serde_json::from_str(raw)?;
    let mut out = Vec::new();
    for block in blocks {
        let instruct = block.parsing_instruct.as_deref().unwrap_or("new_texts");
        match instruct {
            "new_texts" => {
                let texts: Vec<NewText> = serde_json::from_value(block.items)?;
                out.push(ParsedPayload::NewTexts(texts));
            }
            "new_templates" => {
                let templates: Vec<NewTemplate> = serde_json::from_value(block.items)?;
                out.push(ParsedPayload::NewTemplates(templates));
            }
            "new_categories" => {
                let categories: Vec<NewCategory> = serde_json::from_value(block.items)?;
                out.push(ParsedPayload::NewCategories(categories));
            }
            other => {
                return Err(serde_json::Error::custom(format!(
                    "unknown parsing_instruct: {}",
                    other
                )));
            }
        }
    }
    Ok(out)
}
