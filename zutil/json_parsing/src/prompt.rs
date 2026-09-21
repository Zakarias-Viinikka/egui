pub fn prompt() -> &'static str {
    r#"The JSON is a list of blocks. Each block has its own parsing_instruct.

A block:
{
  "parsing_instruct": "new_texts",   // optional, defaults to "new_texts"
  "items": [ ... ]
}

Valid parsing_instruct values: "new_texts", "new_templates", "new_categories".

--- new_texts ---

Each item creates one text:
{
  "title": "required string",
  "body": "required string",
  "category": "optional string",
  "meta_category": "optional string",
  "type_of_text": "optional string"
}

--- new_templates ---

Each item creates one template:
{
  "title": "required string",
  "content": "required string",
  "instructions": "optional string",
  "example": "optional string",
  "category": "optional string",
  "meta_category": "optional string"
}

Templates use %%name%% markers inside content. When filled in, each marker becomes
whatever the user types for it. "example" shows what a filled-in one looks like.
"instructions" says what each marker should contain.

--- new_categories ---

Each item creates one category:
{
  "name": "required string",
  "type_of_category": "normal" or "meta" (optional, defaults to "normal")
}

If a category or meta_category name doesn't exist yet, it gets created automatically when referenced by a text or template.

--- example ---

[
  {
    "items": [
      { "title": "first", "body": "hello", "category": "greetings", "meta_category": "language", "type_of_text": "terminal command" },
      { "title": "second", "body": "world" }
    ]
  },
  {
    "parsing_instruct": "new_templates",
    "items": [
      {
        "title": "greeting template",
        "content": "Hello %%name%%, welcome to %%place%%.",
        "instructions": "name is who you are greeting. place is where they are.",
        "example": "Hello Zakke, welcome to the lab.",
        "category": "greetings"
      }
    ]
  }
]

Categories group related texts. Meta categories group categories more broadly.
"#
}
