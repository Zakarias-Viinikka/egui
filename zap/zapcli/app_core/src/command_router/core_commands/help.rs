use super::super::{Line, Outcome};

pub fn run() -> Outcome {
    Outcome::Print(vec![
        Line::cli("zapcli commands:"),
        Line::normal("  ls [path]           list a directory"),
        Line::normal("  cd <path>           change directory"),
        Line::normal("  cat <path>...       read files (also copies to clipboard)"),
        Line::normal("  create <path>       create a file (chain block: @new ... @end)"),
        Line::normal("  replace <path>      replace text (chain: @old ... @new ... @end)"),
        Line::normal("  overwrite <path>    overwrite a file (chain: @new ... @end)"),
        Line::normal("  append <path>       append to a file (chain: @new ... @end)"),
        Line::normal("  rm <path>...        delete file(s); always needs review"),
        Line::normal("  mv <from> <to>      rename a file; always needs review"),
        Line::normal("  mkdir <path>...     create folder(s)"),
        Line::normal("  cargo <args>        run cargo and stream output (allow-listed)"),
        Line::normal("  run <file> [args]   run one file, with trust check"),
        Line::normal("  zap <text>          send text to zapgui"),
        Line::normal("  whitelist           open zapgui's whitelist menu"),
        Line::normal("  s | settings        show settings"),
        Line::normal("  copy [all]          copy the whole scrollback"),
        Line::normal("  cls | clear         clear the scrollback"),
        Line::normal("  head <file>         show the first lines of a file"),
        Line::normal("  tail <file>         show the last lines of a file"),
        Line::normal("  wc <file>           line, word, byte counts"),
        Line::normal("  last [path]         show the last change zap made to a file"),
        Line::normal("  help                this list"),
        Line::normal("  exit | quit         close zapcli"),
        Line::cli("chain writes need absolute paths or ~/... . A chain may end"),
        Line::cli("with 'cd <folder>' then 'cargo <args>' or 'run <file> [args]'."),
        Line::cli("see ~/ProgStuff/egui/zap/CHAIN_CHEATSHEET.md"),
    ])
}