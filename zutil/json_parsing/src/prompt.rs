pub fn prompt() -> &'static str {
    r#"The JSON is a list of blocks. Each block has its own parsing_instruct.

A block:
{
  "parsing_instruct": "new_texts",   // optional, defaults to "new_texts"
  "items": [ ... ]
}

Each item in "items" becomes one text:
{
  "title": "required string",
  "body": "required string",
  "category": "optional string",
  "meta_category": "optional string",
  "type_of_text": "optional string"
}

--- new_categories ---

Each item creates one category:
{
  "name": "required string",
  "type_of_category": "normal" or "meta" (optional, defaults to "normal")
}

If a category or meta_category name doesn't exist yet, it gets created automatically when referenced by a text or template.

Example:
[
  {
    "items": [
      { "title": "first", "body": "hello", "category": "greetings", "meta_category": "language", "type_of_text": "terminal command" },
      { "title": "second", "body": "world" }
    ]
  }
]

Categories group related texts. Meta categories group categories more broadly.
"#
}
