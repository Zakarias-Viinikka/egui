//! Suggest small corrections to a chain that failed to parse.
//!
//! The parser reports every error it can find, but not what to change.
//! This module reads those errors and, for the safe subset — a
//! misspelled verb, a wrong marker word, a marker typo like `@ends`,
//! a missing final marker, or a missing `@end` before another step —
//! produces a corrected copy of the chain plus a short list of what it
//! changed.
//!
//! The rules are conservative. A fix that might change the meaning of
//! the chain is not proposed. After each round of fixes the corrected
//! text is parsed again; the loop repeats until either the chain parses
//! cleanly or no further fix is available.

use crate::chain::{ParseError, ParseFailure, Verb};
use std::collections::HashSet;

pub struct Repair {
    pub corrected: String,
    pub changes: Vec<String>,
}

/// Suggest a repair for `failure`, given the chain text that produced it.
/// Returns None when no safe fix covers every error, or when the final
/// corrected chain does not parse.
pub fn repair(input: &str, failure: &ParseFailure) -> Option<Repair> {
    let mut lines: Vec<String> = input.lines().map(|s| s.to_string()).collect();
    let trailing_newline = input.ends_with('\n');
    let mut changes: Vec<String> = Vec::new();
    let mut current_failure = failure.clone();

    for _ in 0..4 {
        let mut modified: HashSet<usize> = HashSet::new();
        let mut sorted: Vec<&ParseError> = current_failure.errors().iter().collect();
        sorted.sort_by_key(|e| std::cmp::Reverse(e.line));

        let mut made_progress = false;
        for error in sorted {
            if let Some(change) = repair_one(&mut lines, error, &mut modified) {
                changes.push(change);
                made_progress = true;
            }
        }

        if !made_progress {
            return None;
        }

        let mut corrected = lines.join("\n");
        if trailing_newline {
            corrected.push('\n');
        }

        match crate::chain::parse_with(&corrected, &crate::chain::ParseConfig::empty()) {
            Ok(_) => {
                if corrected == input {
                    return None;
                }
                return Some(Repair { corrected, changes });
            }
            Err(new_failure) => current_failure = new_failure,
        }
    }

    None
}

fn repair_one(
    lines: &mut Vec<String>,
    error: &ParseError,
    modified: &mut HashSet<usize>,
) -> Option<String> {
    let msg = &error.message;
    let line_no = error.line;

    // Misspelled verb.
    if let Some(rest) = msg.strip_prefix("unknown command: ") {
        let word = rest.trim_matches('"');
        let suggestion = closest_verb(word)?;
        if suggestion == word {
            return None;
        }
        let idx = line_no.checked_sub(1)?;
        if idx >= lines.len() || !modified.insert(idx) {
            return None;
        }
        lines[idx] = replace_first_word(&lines[idx], suggestion);
        return Some(format!(
            "change \"{}\" to \"{}\" at line {}",
            word, suggestion, line_no
        ));
    }

    // Wrong marker: "expected @X, got @Y" (from read_block or read_body).
    if let Some((before, after)) = msg.split_once("expected @") {
        if let Some((expected, got_raw)) = after.split_once(", got @") {
            let got = got_raw.strip_prefix('@').unwrap_or(got_raw);

            // Only treat this as a marker typo if `got` is close to a real
            // marker. Otherwise the @-prefixed word is probably intentional
            // content and we shouldn't guess.
            let near_marker = matches!(got, "new" | "old" | "end")
                || ["new", "old", "end"]
                    .iter()
                    .any(|m| levenshtein(got, m) <= 2);
            if !near_marker {
                return None;
            }

            let from_read_block = before.trim_end().ends_with(':');

            if !from_read_block && expected == "end" && (got == "new" || got == "old") {
                // A missing @end before another step. Insert @end before
                // the nearest header-looking line above the found marker.
                let idx = line_no.checked_sub(1)?;
                if idx >= lines.len() {
                    return None;
                }
                for j in (0..idx).rev() {
                    let t = lines[j].trim_end().to_string();
                    if t.is_empty() {
                        continue;
                    }
                    if matches!(t.as_str(), "@new" | "@old" | "@end") {
                        return None;
                    }
                    let first_word = t.split_whitespace().next().unwrap_or("");
                    let is_header = match Verb::from_word(first_word) {
                        Some(v) => !v.is_run(),
                        None => false,
                    };
                    if is_header {
                        if !modified.insert(j) {
                            return None;
                        }
                        lines.insert(j, "@end".to_string());
                        return Some(format!("insert \"@end\" before line {}", j + 1));
                    }
                }
                return None;
            }

            // Replace the wrong marker with the expected one.
            let idx = line_no.checked_sub(1)?;
            if idx >= lines.len() || !modified.insert(idx) {
                return None;
            }
            lines[idx] = format!("@{}", expected);
            return Some(format!(
                "change the marker at line {} to \"@{}\"",
                line_no, expected
            ));
        }

        // "expected @X" with no "got": the block has no body at all.
        // Unsafe to guess what the body should be.
        return None;
    }

    // Missing marker: "missing @X".
    if let Some(expected) = msg.strip_prefix("missing @") {
        let target = format!("@{}", expected);
        // Find a typo of the expected marker anywhere in the chain.
        for i in (0..lines.len()).rev() {
            let t = lines[i].trim_end().to_string();
            if t == target {
                continue;
            }
            if let Some(after_at) = t.strip_prefix('@') {
                if after_at != expected && levenshtein(after_at, expected) <= 2 {
                    if !modified.insert(i) {
                        return None;
                    }
                    lines[i] = target.clone();
                    return Some(format!(
                        "change \"{}\" to \"@{}\" at line {}",
                        t,
                        expected,
                        i + 1
                    ));
                }
            }
        }
        // No typo found: append.
        lines.push(target.clone());
        return Some(format!("append \"{}\" at the end", target));
    }

    None
}

/// The chain verb closest to `word`, if `word` is close enough. The edit
/// distance threshold scales with the word's length, so a short typo like
/// `cut` is not mistaken for `run`.
pub fn closest_verb(word: &str) -> Option<&'static str> {
    if word.len() < 3 {
        return None;
    }
    let threshold = if word.len() >= 6 { 2 } else { 1 };
    let mut best: Option<(&'static str, usize)> = None;
    for v in Verb::ALL.iter() {
        let name = v.name();
        let d = levenshtein(word, name);
        if d <= threshold {
            match best {
                None => best = Some((name, d)),
                Some((_, bd)) if d < bd => best = Some((name, d)),
                _ => {}
            }
        }
    }
    best.map(|(s, _)| s)
}

fn replace_first_word(line: &str, new_word: &str) -> String {
    let trimmed_start = line.len() - line.trim_start().len();
    let leading = &line[..trimmed_start];
    let rest = &line[trimmed_start..];
    if let Some(ws_idx) = rest.find(char::is_whitespace) {
        format!("{}{}{}", leading, new_word, &rest[ws_idx..])
    } else {
        format!("{}{}", leading, new_word)
    }
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let n = a.len();
    let m = b.len();
    if n == 0 {
        return m;
    }
    if m == 0 {
        return n;
    }
    let mut prev: Vec<usize> = (0..=m).collect();
    let mut curr: Vec<usize> = vec![0; m + 1];
    for i in 1..=n {
        curr[0] = i;
        for j in 1..=m {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[m]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::{parse_with, ParseConfig};

    fn fail(input: &str) -> ParseFailure {
        parse_with(input, &ParseConfig::empty()).expect_err("should fail")
    }

    fn repair_ok(input: &str) -> Repair {
        let r = repair(input, &fail(input)).expect("repair");
        parse_with(&r.corrected, &ParseConfig::empty())
            .expect("repaired chain parses cleanly");
        r
    }

    fn repair_none(input: &str) {
        let f = match parse_with(input, &ParseConfig::empty()) {
            Ok(_) => return,
            Err(f) => f,
        };
        assert!(
            repair(input, &f).is_none(),
            "expected no repair for: {input}\nproposal: {:?}",
            repair(input, &f).map(|r| r.corrected)
        );
    }

    // ---- misspelled verb ----

    #[test]
    fn misspelled_verb_gets_a_suggestion() {
        let input = "craete /tmp/a.md\n@new\nx\n@end\n";
        let r = repair_ok(input);
        assert_eq!(r.corrected, "create /tmp/a.md\n@new\nx\n@end\n");
        assert_eq!(r.changes.len(), 1);
        assert!(r.changes[0].contains("craete"), "got: {:?}", r.changes);
        assert!(r.changes[0].contains("create"), "got: {:?}", r.changes);
        assert!(r.changes[0].contains("line 1"), "got: {:?}", r.changes);
    }

    #[test]
    fn two_misspelled_verbs_both_get_fixed() {
        let input = "craete /tmp/a.md\n@new\nx\n@end\ndelet /tmp/b.md\n";
        let r = repair_ok(input);
        assert_eq!(r.changes.len(), 2, "got: {:?}", r.changes);
        assert!(r.corrected.starts_with("create"));
        assert!(r.corrected.contains("delete /tmp/b.md"));
    }

    #[test]
    fn short_word_does_not_match_a_verb() {
        assert!(closest_verb("cr").is_none());
        assert!(closest_verb("cu").is_none());
        assert!(closest_verb("a").is_none());
    }

    #[test]
    fn medium_word_needs_close_match() {
        assert_eq!(closest_verb("creat"), Some("create"));
        assert_eq!(closest_verb("creaet"), Some("create"));
        assert_eq!(closest_verb("creatx"), Some("create"));
        assert!(closest_verb("crexxx").is_none());
    }

    #[test]
    fn long_word_allows_two_edits() {
        assert_eq!(closest_verb("craete"), Some("create"));
        assert_eq!(closest_verb("replcae"), Some("replace"));
        assert_eq!(closest_verb("overwrit"), Some("overwrite"));
    }

    #[test]
    fn unrelated_word_gets_no_suggestion() {
        repair_none("frobnicate /tmp/a.md\n");
        repair_none("xyzzy /tmp/a.md\n");
    }

    // ---- marker typos ----

    #[test]
    fn ends_typo_is_replaced() {
        let input = "create /tmp/a.md\n@new\nx\n@ends\n";
        let r = repair_ok(input);
        assert_eq!(r.corrected, "create /tmp/a.md\n@new\nx\n@end\n");
        assert!(r.changes[0].contains("@ends"), "got: {:?}", r.changes);
        assert!(r.changes[0].contains("line 4"), "got: {:?}", r.changes);
    }

    #[test]
    fn news_typo_is_replaced() {
        let input = "create /tmp/a.md\n@news\nx\n@end\n";
        let r = repair_ok(input);
        assert!(r.corrected.contains("@new\n"));
        assert!(!r.corrected.contains("@news"));
    }

    #[test]
    fn olds_typo_is_replaced() {
        let input = "replace /tmp/a.md\n@olds\nx\n@new\ny\n@end\n";
        let r = repair_ok(input);
        assert!(r.corrected.contains("@old\n"));
        assert!(!r.corrected.contains("@olds"));
    }

    #[test]
    fn nonexistent_marker_word_is_not_guessed() {
        // "@something" isn't a typo of any real marker. Don't repair.
        repair_none("create /tmp/a.md\n@something\nx\n@end\n");
    }

    // ---- missing final marker ----

    #[test]
    fn missing_final_end_is_appended() {
        let input = "create /tmp/a.md\n@new\nhello\n";
        let r = repair_ok(input);
        assert!(r.corrected.ends_with("@end\n"), "got: {:?}", r.corrected);
    }

    #[test]
    fn missing_final_end_with_typo_elsewhere_changes_that() {
        // @ends appears but the block is otherwise fine. The missing-@end
        // path should find the typo and replace it rather than appending.
        let input = "create /tmp/a.md\n@new\nhello\n@ends\n";
        let r = repair_ok(input);
        assert_eq!(r.corrected.matches("@end").count(), 1);
        assert!(!r.corrected.contains("@ends"));
    }

    // ---- missing @end before another step ----

    #[test]
    fn missing_end_between_two_creates_inserts_end_before_header() {
        let input = "create /tmp/a.md\n@new\nalpha\n@end\n\ncreate /tmp/b.md\n@new\nbeta\n\ncreate /tmp/c.md\n@new\ngamma\n@end\n";
        let r = repair_ok(input);
        assert_eq!(r.corrected.matches("@end").count(), 3, "got: {:?}", r.corrected);
        assert!(r.changes.iter().any(|c| c.contains("insert")), "got: {:?}", r.changes);
    }

    #[test]
    fn missing_end_before_second_create_inserts_end() {
        let input = "create /tmp/a.md\n@new\nx\ncreate /tmp/b.md\n@new\ny\n@end\n";
        let r = repair_ok(input);
        assert_eq!(
            r.corrected,
            "create /tmp/a.md\n@new\nx\n@end\ncreate /tmp/b.md\n@new\ny\n@end\n"
        );
    }

    #[test]
    fn missing_end_before_replace_inserts_end() {
        let input = "create /tmp/a.md\n@new\nx\nreplace /tmp/b.md\n@old\np\n@new\nq\n@end\n";
        let r = repair_ok(input);
        assert_eq!(r.corrected.matches("@end").count(), 2);
    }

    #[test]
    fn missing_end_with_swallowed_header_appends() {
        // The parser can't distinguish body text from an abandoned step
        // header for a non-marker verb, so the conservative repair is to
        // append @end. The user can add an intermediate @end by hand.
        let input = "create /tmp/a.md\n@new\nx\ndelete /tmp/b.md\n";
        let r = repair_ok(input);
        assert!(r.corrected.ends_with("@end\n"));
    }

    // ---- header wrong marker ----

    #[test]
    fn create_with_old_marker_replaces_not_inserts() {
        let input = "create /tmp/a.md\n@old\nx\n@end\n";
        let r = repair_ok(input);
        assert_eq!(r.corrected, "create /tmp/a.md\n@new\nx\n@end\n");
        assert!(r.changes[0].contains("change"), "got: {:?}", r.changes);
        assert!(!r.changes[0].contains("insert"), "got: {:?}", r.changes);
    }

    // ---- combined errors ----

    #[test]
    fn misspelled_verb_plus_typo_marker() {
        let input = "craete /tmp/a.md\n@new\nx\n@ends\n";
        let r = repair_ok(input);
        assert!(r.corrected.starts_with("create"));
        assert!(r.corrected.ends_with("@end\n"));
        assert_eq!(r.changes.len(), 2, "got: {:?}", r.changes);
    }

    #[test]
    fn misspelled_verb_plus_missing_end() {
        let input = "craete /tmp/a.md\n@new\nx\n";
        let r = repair_ok(input);
        assert!(r.corrected.starts_with("create"));
        assert!(r.corrected.ends_with("@end\n"));
        assert_eq!(r.changes.len(), 2, "got: {:?}", r.changes);
    }

    // ---- unrecoverable errors ----

    #[test]
    fn relative_path_is_not_repaired() {
        repair_none("create src/a.md\n@new\nx\n@end\n");
    }

    #[test]
    fn double_at_new_alone_is_not_repaired() {
        // Missing @end on the first block but no way to tell where the
        // second @new belongs. Don't guess.
        repair_none("create /tmp/a.md\n@new\nx\n@new\ny\n@end\n");
    }

    #[test]
    fn missing_new_in_replace_is_not_repaired() {
        // A replace without a @new section can't be fixed by guesswork.
        repair_none("replace /tmp/a.md\n@old\nx\n@end\n");
    }

    #[test]
    fn missing_old_in_replace_is_not_repaired() {
        // Same reasoning.
        repair_none("replace /tmp/a.md\n@new\nx\n@end\n");
    }

    // ---- newline preservation ----

    #[test]
    fn preserves_trailing_newline() {
        let input = "craete /tmp/a.md\n@new\nx\n@end\n";
        let r = repair_ok(input);
        assert!(r.corrected.ends_with('\n'));
    }

    #[test]
    fn preserves_no_trailing_newline() {
        let input = "craete /tmp/a.md\n@new\nx\n@end";
        let r = repair_ok(input);
        assert!(!r.corrected.ends_with('\n'));
    }

    // ---- helpers ----

    #[test]
    fn levenshtein_basics() {
        assert_eq!(levenshtein("", ""), 0);
        assert_eq!(levenshtein("a", ""), 1);
        assert_eq!(levenshtein("", "ab"), 2);
        assert_eq!(levenshtein("craete", "create"), 2);
        assert_eq!(levenshtein("end", "end"), 0);
        assert_eq!(levenshtein("ends", "end"), 1);
    }

    #[test]
    fn no_repair_when_nothing_actually_changes() {
        let input = "craete /tmp/a.md\n@new\nx\n@end\n";
        let err = crate::chain::ParseError {
            line: 1,
            message: "unknown command: \"create\"".to_string(),
        };
        let f = crate::chain::ParseFailure::from_errors(vec![err]).expect("non-empty");
        assert!(repair(input, &f).is_none());
    }
}