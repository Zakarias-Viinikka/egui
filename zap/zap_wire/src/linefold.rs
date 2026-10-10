//! Fold a stream chunk into finished lines and one in-progress line.
//!
//! \n finishes the current line. \r marks the current line for replacement:
//! the next character starts it over. In a real terminal \r only moves the
//! cursor, the text stays visible until something overwrites it. Cargo
//! draws its progress bar, then \r, then rewrites on the next redraw.

#[derive(Default, Clone)]
pub struct LineFolder {
    pub lines: Vec<String>,
    pub current: String,
    cr_pending: bool,
}

impl LineFolder {
    pub fn push_str(&mut self, chunk: &str) {
        for ch in chunk.chars() {
            match ch {
                '\r' => self.cr_pending = true,
                '\n' => {
                    self.lines.push(std::mem::take(&mut self.current));
                    self.cr_pending = false;
                }
                c => {
                    if self.cr_pending {
                        self.current.clear();
                        self.cr_pending = false;
                    }
                    self.current.push(c);
                }
            }
        }
    }

    pub fn finish(&mut self) {
        if !self.current.is_empty() {
            self.lines.push(std::mem::take(&mut self.current));
        }
        self.cr_pending = false;
    }

    pub fn clear(&mut self) {
        self.lines.clear();
        self.current.clear();
        self.cr_pending = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cr_before_next_char_keeps_text() {
        let mut f = LineFolder::default();
        f.push_str("a\rb\n");
        assert_eq!(f.lines, vec!["b".to_string()]);
    }

    #[test]
    fn trailing_cr_leaves_current_visible() {
        let mut f = LineFolder::default();
        f.push_str("bar\r");
        assert!(f.lines.is_empty());
        assert_eq!(f.current, "bar");
    }

    #[test]
    fn bar_then_next_overwrites() {
        let mut f = LineFolder::default();
        f.push_str("bar\rnext\n");
        assert_eq!(f.lines, vec!["next".to_string()]);
    }

    #[test]
    fn cr_then_newline_finishes_current() {
        let mut f = LineFolder::default();
        f.push_str("x\r\ny\n");
        assert_eq!(f.lines, vec!["x".to_string(), "y".to_string()]);
    }
}
