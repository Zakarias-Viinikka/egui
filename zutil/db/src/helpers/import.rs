use db_wrapper::mascot::LiveForever;
use json_parsing::parse::ParsedPayload;
use protocol::error::DbError;

use crate::helpers::category::category_id_opt;
use crate::helpers::new_row::{
    create_template_from_new_template, create_text_from_new_text,
};

pub fn import_payloads(
    db: &LiveForever,
    payloads: Vec<ParsedPayload>,
) -> Result<(usize, usize, usize), DbError> {
    db.begin_all_or_nothing()?;

    let mut text_count = 0usize;
    let mut tpl_count = 0usize;
    let mut cat_count = 0usize;

    let result = (|| -> Result<(), DbError> {
        for payload in payloads {
            match payload {
                ParsedPayload::NewTexts(items) => {
                    for nt in items {
                        let row = create_text_from_new_text(db, nt)?;
                        db.insert_data(row)?;
                        text_count += 1;
                    }
                }
                ParsedPayload::NewTemplates(items) => {
                    for nt in items {
                        let row = create_template_from_new_template(db, nt)?;
                        db.insert_data(row)?;
                        tpl_count += 1;
                    }
                }
                ParsedPayload::NewCategories(items) => {
                    for c in items {
                        if category_id_opt(db, &c.name, &c.type_of_category)?.is_some() {
                            cat_count += 1;
                        }
                    }
                }
            }
        }
        Ok(())
    })();

    match result {
        Ok(()) => {
            db.everything_went_perfectly()?;
            Ok((text_count, tpl_count, cat_count))
        }
        Err(e) => {
            let _ = db.regret_everything();
            Err(e)
        }
    }
}
