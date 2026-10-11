//! Token-level similarity between two lines. Shared by edit-vs-replace
//! (pair.rs) and move detection (moves.rs).
//!
//! The unit is a run of letters, digits or underscore. Punctuation and
//! whitespace are ignored so that unrelated lines don't score highly just
//! because they both have a semicolon at the end. Each token's weight is
//! its character count. The score is 2 * matched weight / (weight of both
//! lines), in 0..=1.

pub const SIMILARITY_THRESHOLD: f32 = 0.5;

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
    fn dice_is_one_for_identical_tokens() {
        assert_eq!(dice_over_tokens("a b c", "a b c"), 1.0);
    }

    #[test]
    fn dice_is_zero_for_disjoint_tokens() {
        assert_eq!(dice_over_tokens("aaa bbb", "ccc ddd"), 0.0);
    }
}
