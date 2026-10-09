use super::super::{Line, Outcome};

pub fn run() -> Outcome {
    Outcome::Print(vec![
        Line::cli("zapcli commands:"),
        Line::normal("  ls [path]           list a directory"),
        Line::normal("  cd <path>           change directory"),
        Line::normal("  cat <path>          read a file (also copies to clipboard)"),
        Line::normal("  create <path>       create a file (chain block: @new ... @end)"),
        Line::normal("  replace <path>      replace text (chain: @old ... @new ... @end)"),
        Line::normal("  overwrite <path>    overwrite a file (chain: @new ... @end)"),
        Line::normal("  zap <text>          send text to zapgui"),
        Line::normal("  whitelist           open zapgui's whitelist menu"),
        Line::normal("  s | settings        show settings"),
        Line::normal("  copy                copy the whole scrollback"),
        Line::normal("  cls | clear         clear the scrollback"),
        Line::normal("  help                this list"),
        Line::normal("  exit | quit         close zapcli"),
        Line::cli("chains: see ~/ProgStuff/egui/zap/CHAIN_CHEATSHEET.md"),
    ])
}
