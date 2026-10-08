pub fn prompt(existing_categories: &[(String, String)]) -> String {
    let cat_list = if existing_categories.is_empty() {
        "  (none yet — you can create the first one, but see the rules below)".to_string()
    } else {
        existing_categories
            .iter()
            .map(|(name, ty)| format!("  - {:?} ({})", name, ty))
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        r#"You are filling zutil with data. zutil has three top-level menu screens: AI
Prompts, Terminal Commands, and Text. Where a text ends up is decided by
`type_of_text` and `category`. Getting these right matters — a text with the
wrong type will appear in the wrong menu, and a category with the wrong case
will be rejected.

The JSON is a list of blocks. Each block has its own parsing_instruct.

A block:
{{
  "parsing_instruct": "new_texts",   // optional, defaults to "new_texts"
  "items": [ ... ]
}}

Valid parsing_instruct values: "new_texts", "new_templates", "new_categories".

--- new_texts ---

Each item creates one text:
{{
  "title": "required string",
  "body": "required string",
  "category": "optional string",
  "meta_category": "optional string",
  "type_of_text": "optional string",
  "copy_instead_of_view": "optional bool, default false"
}}

### type_of_text — this decides which screen the text lands in

- "prompt"           -> AI Prompts
- "terminal command" -> Terminal Commands
- anything else, or empty -> Text

### category — this is the submenu the text sits under

The category name must match an existing category in the DB. Matching is
case-insensitive on the ASCII letters, but if you write a different case than
what's stored you get an error and the import fails. Use the exact case shown
in the list below.

If the category you want doesn't exist yet, you may create it by referencing
it — but only if no case-variant of it already exists. "AI PROMPTS" fails
because "ai prompts" exists. Pick the existing one or invent a truly new name.

Current categories in the DB:

{cat_list}

### meta_category — a broader grouping. Optional.

Usually leave it empty. If the existing rows for the same kind of text use a
meta category, match it. Same rules as category: must match an existing meta
category case-insensitively, or be a genuinely new name.

### copy_instead_of_view

If true, clicking this text in a circle nav copies it to the clipboard instead
of opening the view overlay. Use this for prompts and terminal commands. Defaults
to false (view) — that's for notes and general text.

--- new_templates ---

Each item creates one template:
{{
  "title": "required string",
  "content": "required string",
  "instructions": "optional string",
  "example": "optional string",
  "category": "optional string",
  "meta_category": "optional string"
}}

Templates use %%name%% markers inside content. When filled in, each marker becomes
whatever the user types for it. "example" shows what a filled-in one looks like.
"instructions" says what each marker should contain.

Templates use the same category / meta_category rules as texts.

--- new_categories ---

Each item creates one category:
{{
  "name": "required string",
  "type_of_category": "normal" or "meta" (optional, defaults to "normal")
}}

You usually don't need this block — categories are created on demand when a
text or template references them by name. Only use it when you want to create
a category with no text in it yet.

--- worked example ---

A prompt about how to explain code to the user:

[
  {{
    "items": [
      {{
        "title": "How I want explanations",
        "body": "...",
        "category": "ai prompts",
        "meta_category": "ai",
        "type_of_text": "prompt",
        "copy_instead_of_view": true
      }}
    ]
  }}
]

A terminal command:

[
  {{
    "items": [
      {{
        "title": "last 10 commits",
        "body": "git log --oneline -10",
        "category": "git",
        "type_of_text": "terminal command",
        "copy_instead_of_view": true
      }}
    ]
  }}
]

A note (no special type, lands in Text):

[
  {{
    "items": [
      {{
        "title": "where the androidTest apk gets built",
        "body": "...",
        "category": "android",
        "type_of_text": "notes"
      }}
    ]
  }}
]

--- the mistakes to avoid ---

1. Writing a category name with different case than what's stored. It fails.
2. Forgetting type_of_text on something that's clearly a prompt. It lands in
   Text instead of AI Prompts, and you won't find it where you expect.
3. Sending multiple blocks without "parsing_instruct" — the default is
   "new_texts", so a templates block without it will try to parse as texts and
   fail on missing required fields.
4. Nested quotes inside body strings — escape them, or the JSON is invalid."#
    )
}
