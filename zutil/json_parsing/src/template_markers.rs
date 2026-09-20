pub fn extract_markers(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = content;
    while let Some(open) = rest.find("%%") {
        let after_open = &rest[open + 2..];
        match after_open.find("%%") {
            Some(close) => {
                let name = &after_open[..close];
                if !name.is_empty() && !out.iter().any(|m: &String| m == name) {
                    out.push(name.to_string());
                }
                rest = &after_open[close + 2..];
            }
            None => break,
        }
    }
    out
}

pub fn fill_markers(content: &str, values: &std::collections::HashMap<String, String>) -> String {
    let mut out = String::new();
    let mut rest = content;
    loop {
        match rest.find("%%") {
            Some(open) => {
                out.push_str(&rest[..open]);
                let after_open = &rest[open + 2..];
                match after_open.find("%%") {
                    Some(close) => {
                        let name = &after_open[..close];
                        match values.get(name) {
                            Some(v) => out.push_str(v),
                            None => {
                                out.push_str("%%");
                                out.push_str(name);
                                out.push_str("%%");
                            }
                        }
                        rest = &after_open[close + 2..];
                    }
                    None => {
                        out.push_str("%%");
                        out.push_str(after_open);
                        return out;
                    }
                }
            }
            None => {
                out.push_str(rest);
                return out;
            }
        }
    }
}
