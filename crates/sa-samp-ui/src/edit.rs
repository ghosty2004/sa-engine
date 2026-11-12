//! A single-line edit box (the chat input and the dialogs' input line): caret editing, capacity and the
//! password mask.

/// Editing keys (Windows virtual-key codes are mapped in [`crate::ui`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditKey {
    /// Caret left.
    Left,
    /// Caret right.
    Right,
    /// Caret to the start.
    Home,
    /// Caret to the end.
    End,
    /// Delete before the caret.
    Backspace,
    /// Delete after the caret.
    Delete,
}

/// The edit state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LineEdit {
    text: Vec<char>,
    caret: usize,
    capacity: usize,
}

impl LineEdit {
    /// An empty edit taking at most `capacity` characters.
    pub fn new(capacity: usize) -> Self {
        Self {
            text: Vec::new(),
            caret: 0,
            capacity,
        }
    }

    /// The text.
    pub fn text(&self) -> String {
        self.text.iter().collect()
    }

    /// Caret position in characters.
    pub fn caret(&self) -> usize {
        self.caret
    }

    /// Whether the text is empty.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Replaces the text, caret at the end (history recall).
    pub fn set_text(&mut self, t: &str) {
        self.text = t.chars().take(self.capacity).collect();
        self.caret = self.text.len();
    }

    /// Clears the text.
    pub fn clear(&mut self) {
        self.text.clear();
        self.caret = 0;
    }

    /// Types a character; control characters and a full edit are ignored. Returns whether it was taken.
    pub fn insert(&mut self, c: char) -> bool {
        if c.is_control() || self.text.len() >= self.capacity {
            return false;
        }
        self.text.insert(self.caret, c);
        self.caret += 1;
        true
    }

    /// An editing key.
    pub fn key(&mut self, k: EditKey) {
        match k {
            EditKey::Left => self.caret = self.caret.saturating_sub(1),
            EditKey::Right => self.caret = (self.caret + 1).min(self.text.len()),
            EditKey::Home => self.caret = 0,
            EditKey::End => self.caret = self.text.len(),
            EditKey::Backspace => {
                if self.caret > 0 {
                    self.caret -= 1;
                    self.text.remove(self.caret);
                }
            }
            EditKey::Delete => {
                if self.caret < self.text.len() {
                    self.text.remove(self.caret);
                }
            }
        }
    }

    /// The text as drawn: `*` per character for password edits.
    pub fn display(&self, password: bool) -> String {
        if password {
            "*".repeat(self.text.len())
        } else {
            self.text()
        }
    }

    /// The displayed text up to the caret (for the caret's x).
    pub fn display_before_caret(&self, password: bool) -> String {
        if password {
            "*".repeat(self.caret)
        } else {
            self.text[..self.caret].iter().collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caret_editing() {
        let mut e = LineEdit::new(5);
        for c in "abc".chars() {
            assert!(e.insert(c));
        }
        e.key(EditKey::Left);
        e.insert('X');
        assert_eq!((e.text(), e.caret()), ("abXc".into(), 3));
        e.key(EditKey::Home);
        e.key(EditKey::Delete);
        assert_eq!(e.text(), "bXc");
        e.key(EditKey::End);
        e.key(EditKey::Backspace);
        assert_eq!((e.text(), e.caret()), ("bX".into(), 2));
        e.key(EditKey::Right);
        assert_eq!(e.caret(), 2);
    }

    #[test]
    fn capacity_control_chars_and_password() {
        let mut e = LineEdit::new(3);
        assert!(!e.insert('\r'));
        for c in "abcd".chars() {
            e.insert(c);
        }
        assert_eq!(e.text(), "abc");
        assert_eq!(e.display(true), "***");
        e.key(EditKey::Left);
        assert_eq!(e.display_before_caret(true), "**");
        e.set_text("hello");
        assert_eq!((e.text(), e.caret()), ("hel".into(), 3));
    }
}
