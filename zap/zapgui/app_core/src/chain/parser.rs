use super::step::{Step, Verb};

#[derive(Debug, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ParseError {}

pub fn parse(input: &str) -> Result<Vec<Step>, ParseError> {
    let lines: Vec<&str> = input.lines().collect();
    let mut steps: Vec<Step> = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        if lines[i].trim_end().is_empty() {
            i += 1;
            continue;
        }
        let header_no = i + 1;
        let (verb, rest) = parse_header(lines[i].trim_end(), header_no)?;
        i += 1;

        if i >= lines.len() {
            return Err(ParseError { line: i + 1, message: "expected a marker line".to_string() });
        }
        let marker_count = count_ats(lines[i]);
        if marker_count == 0 {
            return Err(ParseError { line: i + 1, message: "expected a marker line".to_string() });
        }

        let (old_text, new_text, next_i) = match verb {
            Verb::Replace => {
                let name = marker_name(lines[i]);
                if name != "old" {
                    return Err(ParseError { line: i + 1, message: format!("expected @old, got @{}", name) });
                }
                let (old_text, after_old) = read_until_marker(&lines, i + 1, marker_count, "new")?;
                let (new_text, after_new) = read_until_marker(&lines, after_old, marker_count, "end")?;
                (old_text, new_text, after_new)
            }
            _ => {
                let name = marker_name(lines[i]);
                if name != "new" {
                    return Err(ParseError { line: i + 1, message: format!("expected @new, got @{}", name) });
                }
                let (new_text, after_new) = read_until_marker(&lines, i + 1, marker_count, "end")?;
                (String::new(), new_text, after_new)
            }
        };

        let (path, count) = parse_path_and_count(rest, header_no, verb)?;

        let new_text = if matches!(verb, Verb::Create) {
            let mut t = new_text;
            if !t.ends_with('\n') {
                t.push('\n');
            }
            t
        } else {
            new_text
        };

        steps.push(Step { verb, path, old_text, new_text, count });
        i = next_i;
    }

    Ok(steps)
}

fn parse_header(line: &str, line_no: usize) -> Result<(Verb, &str), ParseError> {
    let (word, rest) = match line.find(char::is_whitespace) {
        Some(idx) => (&line[..idx], &line[idx + 1..]),
        None => (line, ""),
    };
    let verb = match word {
        "create" => Verb::Create,
        "replace" => Verb::Replace,
        "overwrite" => Verb::Overwrite,
        _ => return Err(ParseError { line: line_no, message: format!("unknown command: {:?}", word) }),
    };
    if rest.trim().is_empty() {
        return Err(ParseError { line: line_no, message: format!("missing path after {:?}", word) });
    }
    Ok((verb, rest))
}

fn parse_path_and_count(rest: &str, line: usize, verb: Verb) -> Result<(String, Option<usize>), ParseError> {
    let rest = rest.trim();
    if let Verb::Replace = verb {
        if let Some(idx) = rest.rfind(char::is_whitespace) {
            let tail = &rest[idx + 1..];
            if let Some(digits) = tail.strip_prefix('x') {
                if !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()) {
                    let n: usize = digits.parse().map_err(|_| ParseError {
                        line,
                        message: format!("bad count: {}", tail),
                    })?;
                    let path = rest[..idx].trim();
                    if path.is_empty() {
                        return Err(ParseError { line, message: "missing path".to_string() });
                    }
                    return Ok((path.to_string(), Some(n)));
                }
            }
        }
    }
    if rest.is_empty() {
        return Err(ParseError { line, message: "missing path".to_string() });
    }
    Ok((rest.to_string(), None))
}

fn count_ats(line: &str) -> usize {
    line.chars().take_while(|c| *c == '@').count()
}

fn marker_name(line: &str) -> String {
    let n = count_ats(line);
    line[n..].trim().to_string()
}

fn read_until_marker(
    lines: &[&str],
    start: usize,
    marker_count: usize,
    expected: &str,
) -> Result<(String, usize), ParseError> {
    let mut text = String::new();
    let mut i = start;
    while i < lines.len() {
        if count_ats(lines[i]) == marker_count {
            let name = marker_name(lines[i]);
            if name != expected {
                return Err(ParseError {
                    line: i + 1,
                    message: format!("expected @{}, got @{}", expected, name),
                });
            }
            if text.ends_with('\n') {
                text.pop();
            }
            return Ok((text, i + 1));
        }
        text.push_str(lines[i]);
        text.push('\n');
        i += 1;
    }
    Err(ParseError {
        line: lines.len() + 1,
        message: format!("missing @{}", expected),
    })
}

#[cfg(test)]
mod tests {
    use super::parse;
    use crate::chain::step::Verb;

    #[test]
    fn parses_a_create() {
        let steps = parse("create /tmp/a.rs\n@new\nfn main() {}\n@end\n").expect("parse");
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].verb, Verb::Create);
        assert_eq!(steps[0].path, "/tmp/a.rs");
        assert_eq!(steps[0].new_text, "fn main() {}\n");
        assert_eq!(steps[0].count, None);
    }

    #[test]
    fn parses_a_replace() {
        let steps = parse("replace /tmp/a.rs\n@old\nfoo\n@new\nbar\n@end\n").expect("parse");
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].verb, Verb::Replace);
        assert_eq!(steps[0].old_text, "foo");
        assert_eq!(steps[0].new_text, "bar");
        assert_eq!(steps[0].count, None);
    }

    #[test]
    fn parses_a_replace_with_count() {
        let steps = parse("replace /tmp/a.rs x2\n@old\nfoo\n@new\nbar\n@end\n").expect("parse");
        assert_eq!(steps[0].count, Some(2));
    }

    #[test]
    fn parses_two_steps() {
        let input = "create /tmp/a.rs\n@new\nfoo\n@end\n\nreplace /tmp/a.rs\n@old\nfoo\n@new\nbar\n@end\n";
        let steps = parse(input).expect("parse");
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].verb, Verb::Create);
        assert_eq!(steps[1].verb, Verb::Replace);
    }

    #[test]
    fn escapes_markers_with_extra_at() {
        let input = "create /tmp/a.rs\n@@new\n@new\n@@end\n";
        let steps = parse(input).expect("parse");
        assert_eq!(steps[0].new_text, "@new\n");
    }

    #[test]
    fn errors_on_unknown_verb() {
        let err = parse("frobnicate /tmp/a.rs\n@new\nx\n@end\n").expect_err("should fail");
        assert_eq!(err.line, 1);
    }
}
