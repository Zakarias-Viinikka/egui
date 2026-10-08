use protocol::payload::EditColInRowIn;
use protocol::row_col::Col;

use crate::tables::{tbl_categories, tbl_keyword_lookup, tbl_templates, tbl_texts};

pub fn edit_text_title(text_id: i64, new_title: String) -> EditColInRowIn {
    EditColInRowIn {
        table_name: tbl_texts::TABLE_NAME.to_string(),
        row_id: text_id.to_string(),
        column: tbl_texts::TITLE.name.to_string(),
        new_value: Col::Text(new_title),
    }
}

pub fn edit_text_body(text_id: i64, new_body: String) -> EditColInRowIn {
    EditColInRowIn {
        table_name: tbl_texts::TABLE_NAME.to_string(),
        row_id: text_id.to_string(),
        column: tbl_texts::BODY.name.to_string(),
        new_value: Col::Text(new_body),
    }
}

pub fn edit_keyword_lookup_keyword(keyword_lookup_id: i64, new_keyword: String) -> EditColInRowIn {
    EditColInRowIn {
        table_name: tbl_keyword_lookup::TABLE_NAME.to_string(),
        row_id: keyword_lookup_id.to_string(),
        column: tbl_keyword_lookup::KEYWORD.name.to_string(),
        new_value: Col::Text(new_keyword),
    }
}

pub fn edit_template_title(template_id: i64, new_title: String) -> EditColInRowIn {
    EditColInRowIn {
        table_name: tbl_templates::TABLE_NAME.to_string(),
        row_id: template_id.to_string(),
        column: tbl_templates::TITLE.name.to_string(),
        new_value: Col::Text(new_title),
    }
}

pub fn edit_template_content(template_id: i64, new_content: String) -> EditColInRowIn {
    EditColInRowIn {
        table_name: tbl_templates::TABLE_NAME.to_string(),
        row_id: template_id.to_string(),
        column: tbl_templates::CONTENT.name.to_string(),
        new_value: Col::Text(new_content),
    }
}

pub fn edit_template_instructions(template_id: i64, new_instructions: String) -> EditColInRowIn {
    EditColInRowIn {
        table_name: tbl_templates::TABLE_NAME.to_string(),
        row_id: template_id.to_string(),
        column: tbl_templates::INSTRUCTIONS.name.to_string(),
        new_value: Col::Text(new_instructions),
    }
}


pub fn edit_template_example(template_id: i64, new_example: String) -> EditColInRowIn {
    EditColInRowIn {
        table_name: tbl_templates::TABLE_NAME.to_string(),
        row_id: template_id.to_string(),
        column: tbl_templates::EXAMPLE.name.to_string(),
        new_value: Col::Text(new_example),
    }
}

pub fn edit_category_name(category_id: i64, new_name: String) -> EditColInRowIn {
    EditColInRowIn {
        table_name: tbl_categories::TABLE_NAME.to_string(),
        row_id: category_id.to_string(),
        column: tbl_categories::NAME.name.to_string(),
        new_value: Col::Text(new_name),
    }
}
