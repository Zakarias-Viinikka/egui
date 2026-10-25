//! Token-level similarity between two lines. Shared by edit-vs-replace
//! (pair.rs) and move detection (moves.rs).
//!
//! The unit is a run of letters, digits or underscore. Punctuation and
//! whitespace are ignored so that unrelated lines don't score highly just
//! because they both have a semicolon at the end. Each token's weight is
//! its character count. The score is 2 * matched weight / (weight of both
//! lines), in 0..=1.

pub const SIMILARITY_THRESHOLD: f32 = 0.5;

/// A Change row at or above this score is an edit (drawn blue). Between
/// SIMILARITY_THRESHOLD and this it is a paired replacement.
pub const EDIT_THRESHOLD: f32 = 0.6;

use std::ops::Range;

use similar::{Algorithm, DiffOp};

const INLINE_MAX_CHARS: usize = 2000;

/// Cut a line into tokens: a run of letters, digits and underscore, a run
/// of whitespace, or any single other character. Unlike `tokenize`, this
/// keeps punctuation so an added comma can be marked.
fn token_ranges(text: &str) -> Vec<Range<usize>> {
    let mut out: Vec<Range<usize>> = Vec::new();
    let mut run: Option<(usize, u8)> = None;
    for (i, c) in text.char_indices() {
        let class: u8 = if c.is_alphanumeric() || c == '_' {
            1
        } else if c.is_whitespace() {
            2
        } else {
            0
        };
        match run {
            Some((_, k)) if k == class && class != 0 => {}
            Some((s, _)) => {
                out.push(s..i);
                run = Some((i, class));
            }
            None => run = Some((i, class)),
        }
    }
    if let Some((s, _)) = run {
        out.push(s..text.len());
    }
    out
}

fn span_of(tokens: &[Range<usize>], index: usize, len: usize) -> Option<Range<usize>> {
    if len == 0 {
        return None;
    }
    let first = tokens.get(index)?;
    let last = tokens.get(index + len - 1)?;
    Some(first.start..last.end)
}

fn merge_ranges(mut v: Vec<Range<usize>>) -> Vec<Range<usize>> {
    v.sort_by_key(|r| r.start);
    let mut out: Vec<Range<usize>> = Vec::new();
    for r in v {
        match out.last_mut() {
            Some(last) if r.start <= last.end => {
                if r.end > last.end {
                    last.end = r.end;
                }
            }
            _ => out.push(r),
        }
    }
    out
}

fn mostly_covered(text: &str, spans: &[Range<usize>]) -> bool {
    let total = text.chars().filter(|c| !c.is_whitespace()).count();
    if total == 0 {
        return false;
    }
    let covered: usize = spans
        .iter()
        .map(|r| {
            text.get(r.clone())
                .map(|t| t.chars().filter(|c| !c.is_whitespace()).count())
                .unwrap_or(0)
        })
        .sum();
    covered * 10 > total * 7
}

/// Byte ranges that differ between two lines, one list per side. Empty
/// when either line is over 2,000 characters, or when the marks cover more
/// than 70% of the non-space characters of either line.
pub fn inline_spans(old: &str, new: &str) -> (Vec<Range<usize>>, Vec<Range<usize>>) {
    if old == new
        || old.chars().count() > INLINE_MAX_CHARS
        || new.chars().count() > INLINE_MAX_CHARS
    {
        return (Vec::new(), Vec::new());
    }
    let ta = token_ranges(old);
    let tb = token_ranges(new);
    let sa: Vec<&str> = ta.iter().map(|r| &old[r.clone()]).collect();
    let sb: Vec<&str> = tb.iter().map(|r| &new[r.clone()]).collect();

    let mut left: Vec<Range<usize>> = Vec::new();
    let mut right: Vec<Range<usize>> = Vec::new();
    for op in similar::capture_diff_slices(Algorithm::Myers, &sa, &sb) {
        match op {
            DiffOp::Equal { .. } => {}
            DiffOp::Delete {
                old_index, old_len, ..
            } => {
                if let Some(r) = span_of(&ta, old_index, old_len) {
                    left.push(r);
                }
            }
            DiffOp::Insert {
                new_index, new_len, ..
            } => {
                if let Some(r) = span_of(&tb, new_index, new_len) {
                    right.push(r);
                }
            }
            DiffOp::Replace {
                old_index,
                old_len,
                new_index,
                new_len,
            } => {
                let (Some(o), Some(n)) = (
                    span_of(&ta, old_index, old_len),
                    span_of(&tb, new_index, new_len),
                ) else {
                    continue;
                };
                let ot = &old[o.clone()];
                let nt = &new[n.clone()];
                let pre: usize = ot
                    .chars()
                    .zip(nt.chars())
                    .take_while(|(a, b)| a == b)
                    .map(|(a, _)| a.len_utf8())
                    .sum();
                let ot2 = &ot[pre..];
                let nt2 = &nt[pre..];
                let suf: usize = ot2
                    .chars()
                    .rev()
                    .zip(nt2.chars().rev())
                    .take_while(|(a, b)| a == b)
                    .map(|(a, _)| a.len_utf8())
                    .sum();
                let lr = (o.start + pre)..(o.end - suf);
                let rr = (n.start + pre)..(n.end - suf);
                if lr.start < lr.end {
                    left.push(lr);
                }
                if rr.start < rr.end {
                    right.push(rr);
                }
            }
        }
    }

    let left = merge_ranges(left);
    let right = merge_ranges(right);
    if mostly_covered(old, &left) || mostly_covered(new, &right) {
        return (Vec::new(), Vec::new());
    }
    (left, right)
}

pub fn tokenize(text: &str) -> Vec<&str> {
    let mut tokens: Vec<&str> = Vec::new();
    let mut start: Option<usize> = None;
    for (i, c) in text.char_indices() {
        let is_tok = c.is_alphanumeric() || c == '_';
        match (is_tok, start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                tokens.push(&text[s..i]);
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        tokens.push(&text[s..]);
    }
    tokens
}

/// Token-level similarity of two lines. Scores 1.0 for identical token
/// lists of the same total weight; 0.0 for disjoint ones.
pub fn similarity(a: &str, b: &str) -> f32 {
    if a == b {
        return 1.0;
    }

    let ta = tokenize(a);
    let tb = tokenize(b);
    let wa: Vec<usize> = ta.iter().map(|t| t.chars().count()).collect();
    let wb: Vec<usize> = tb.iter().map(|t| t.chars().count()).collect();
    let total_a: usize = wa.iter().sum();
    let total_b: usize = wb.iter().sum();

    if total_a == 0 && total_b == 0 {
        let ac: Vec<char> = a.chars().filter(|c| !c.is_whitespace()).collect();
        let bc: Vec<char> = b.chars().filter(|c| !c.is_whitespace()).collect();
        return if ac == bc { 1.0 } else { 0.0 };
    }
    if ta.is_empty() || tb.is_empty() {
        return 0.0;
    }

    let matched = weighted_lcs(&ta, &wa, &tb, &wb);
    2.0 * matched as f32 / (total_a + total_b) as f32
}

/// Weighted longest common subsequence of the token lists, matched on
/// equality. Returns the sum of the weights of the matched tokens.
pub fn weighted_lcs(ta: &[&str], wa: &[usize], tb: &[&str], _wb: &[usize]) -> usize {
    let k = ta.len();
    let j = tb.len();
    let mut dp = vec![vec![0usize; j + 1]; k + 1];
    for i in 1..=k {
        for jj in 1..=j {
            let mut best = dp[i - 1][jj].max(dp[i][jj - 1]);
            if ta[i - 1] == tb[jj - 1] {
                best = best.max(dp[i - 1][jj - 1] + wa[i - 1]);
            }
            dp[i][jj] = best;
        }
    }
    dp[k][j]
}

/// Dice score over the multisets of tokens. Used by move detection as a
/// cheap pre-check before running the LCS.
pub fn dice_over_tokens(a: &str, b: &str) -> f32 {
    let ta = tokenize(a);
    let tb = tokenize(b);
    if ta.is_empty() && tb.is_empty() {
        return 0.0;
    }
    let mut used = vec![false; tb.len()];
    let mut common = 0usize;
    for t in &ta {
        for (i, u) in tb.iter().enumerate() {
            if !used[i] && *t == *u {
                used[i] = true;
                common += 1;
                break;
            }
        }
    }
    let total = ta.len() + tb.len();
    if total == 0 {
        return 0.0;
    }
    2.0 * common as f32 / total as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comma_added_is_similar() {
        let s = similarity("let x = 1;", "let x = 1,;");
        assert!(s >= SIMILARITY_THRESHOLD, "got {s}");
    }

    #[test]
    fn renamed_short_variable_is_similar() {
        let s = similarity("let x = 1;", "let y = 1;");
        assert!(s >= SIMILARITY_THRESHOLD, "got {s}");
    }

    #[test]
    fn reindent_is_similar() {
        let s = similarity("        foo();", "    foo();");
        assert!(s >= SIMILARITY_THRESHOLD, "got {s}");
    }

    #[test]
    fn unrelated_lines_are_not_similar() {
        let s = similarity("fn main() {", "let result = compute();");
        assert!(s < SIMILARITY_THRESHOLD, "got {s}");
    }

    #[test]
    fn identical_lines_score_one() {
        assert_eq!(similarity("hello world", "hello world"), 1.0);
    }

    #[test]
    fn punctuation_only_lines_compare_by_chars() {
        assert_eq!(similarity("}}}", "}}}"), 1.0);
        assert!(similarity("}}}", "}}}()") < 1.0);
    }

    #[test]
    fn edit_threshold_is_above_similarity_threshold() {
        assert!(EDIT_THRESHOLD > SIMILARITY_THRESHOLD);
    }

    #[test]
    fn small_changes_score_as_edits() {
        assert!(similarity("let x = 1;", "let y = 1;") >= EDIT_THRESHOLD);
        assert!(similarity("let x = 1;", "let x = 1,;") >= EDIT_THRESHOLD);
    }

    #[test]
    fn half_matching_line_pairs_but_is_not_an_edit() {
        let s = similarity("aaaaa bbbbb ccccc ddddd", "aaaaa bbbbb xxxxx yyyyy");
        assert!(s >= SIMILARITY_THRESHOLD, "got {s}");
        assert!(s < EDIT_THRESHOLD, "got {s}");
    }

    #[test]
    fn spans_swapped_word() {
        let (l, r) = inline_spans("call(foo, 1)", "call(bar, 1)");
        assert_eq!(l.len(), 1);
        assert_eq!(r.len(), 1);
        assert_eq!(&"call(foo, 1)"[l[0].clone()], "foo");
        assert_eq!(&"call(bar, 1)"[r[0].clone()], "bar");
    }

    #[test]
    fn spans_one_added_character() {
        let old = "let a = compute_total(x);";
        let new = "let a = compute_totals(x);";
        let (l, r) = inline_spans(old, new);
        assert!(l.is_empty());
        assert_eq!(r.len(), 1);
        assert_eq!(&new[r[0].clone()], "s");
    }

    #[test]
    fn spans_added_comma() {
        let new = "let x = 1,;";
        let (l, r) = inline_spans("let x = 1;", new);
        assert!(l.is_empty());
        assert_eq!(r.len(), 1);
        assert_eq!(&new[r[0].clone()], ",");
    }

    #[test]
    fn spans_reindent_marks_spaces() {
        let old = "        foo();";
        let (l, r) = inline_spans(old, "    foo();");
        assert_eq!(l.len(), 1);
        assert_eq!(&old[l[0].clone()], "    ");
        assert!(r.is_empty());
    }

    #[test]
    fn spans_identical_lines_give_nothing() {
        let (l, r) = inline_spans("same", "same");
        assert!(l.is_empty() && r.is_empty());
    }

    #[test]
    fn spans_mostly_rewritten_line_gives_nothing() {
        let (l, r) = inline_spans("alpha beta gamma", "one two three");
        assert!(l.is_empty() && r.is_empty());
    }

    #[test]
    fn spans_multibyte_text_stays_on_char_boundaries() {
        let old = "let \u{e5} = 1;";
        let new = "let \u{f6} = 1;";
        let (l, r) = inline_spans(old, new);
        assert!(!l.is_empty() && !r.is_empty());
        assert!(l.iter().all(|x| old.get(x.clone()).is_some()));
        assert!(r.iter().all(|x| new.get(x.clone()).is_some()));
    }

    #[test]
    fn dice_is_one_for_identical_tokens() {
        assert_eq!(dice_over_tokens("a b c", "a b c"), 1.0);
    }

    #[test]
    fn dice_is_zero_for_disjoint_tokens() {
        assert_eq!(dice_over_tokens("aaa bbb", "ccc ddd"), 0.0);
    }
}
