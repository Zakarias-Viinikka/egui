use protocol::payload::EditColInRowIn;
use protocol::row_col::Col;

pub fn edit_text_title(text_id: i64, new_title: String) -> EditColInRowIn {
    EditColInRowIn {
        table_name: "texts".to_string(),
        row_id: text_id.to_string(),
        column: "title".to_string(),
        new_value: Col::Text(new_title),
    }
}

pub fn edit_text_body(text_id: i64, new_body: String) -> EditColInRowIn {
    EditColInRowIn {
        table_name: "texts".to_string(),
        row_id: text_id.to_string(),
        column: "body".to_string(),
        new_value: Col::Text(new_body),
    }
}

pub fn edit_keyword_lookup_keyword(keyword_lookup_id: i64, new_keyword: String) -> EditColInRowIn {
    EditColInRowIn {
        table_name: "keyword_lookup".to_string(),
        row_id: keyword_lookup_id.to_string(),
        column: "keyword".to_string(),
        new_value: Col::Text(new_keyword),
    }
}

pub fn edit_template_title(template_id: i64, new_title: String) -> EditColInRowIn {
    EditColInRowIn {
        table_name: "templates".to_string(),
        row_id: template_id.to_string(),
        column: "title".to_string(),
        new_value: Col::Text(new_title),
    }
}

pub fn edit_template_content(template_id: i64, new_content: String) -> EditColInRowIn {
    EditColInRowIn {
        table_name: "templates".to_string(),
        row_id: template_id.to_string(),
        column: "content".to_string(),
        new_value: Col::Text(new_content),
    }
}

pub fn edit_template_instructions(template_id: i64, new_instructions: String) -> EditColInRowIn {
    EditColInRowIn {
        table_name: "templates".to_string(),
        row_id: template_id.to_string(),
        column: "instructions".to_string(),
        new_value: Col::Text(new_instructions),
    }
}


pub fn edit_template_example(template_id: i64, new_example: String) -> EditColInRowIn {
    EditColInRowIn {
        table_name: "templates".to_string(),
        row_id: template_id.to_string(),
        column: "example".to_string(),
        new_value: Col::Text(new_example),
    }
}
