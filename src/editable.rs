use std::ops::{AddAssign, SubAssign};

use crossterm::event::{KeyCode, KeyEvent};
use tui::{
    style::{Modifier, Style},
    text::Span,
};

pub const EDITING_STR_FOOTER: &[&str] = &["[Esc] Restore", "[Enter] Save"];
pub const EDITING_BOOL_FOOTER: &[&str] = &["[Esc] Stop editing", "[Enter] Toggle"];

pub trait EditableWidget {
    /// Returns the contents of the widget as a string
    fn as_str(&self) -> &str;
    /// Returns the contents of the widget as spans
    fn as_spans(&self, is_editing: bool) -> Vec<Span>;
    /// Key events forwarded from the main loop when editing the widget
    fn handle_event(&mut self, key: &KeyEvent) -> Option<bool> {
        default_handle_event(key)
    }
    /// If the contents have been modified
    fn is_changed(&self) -> bool;
    /// Whether this widget can be edited
    fn is_editable(&self) -> bool;
    /// Returns footer information for when the widget is in edit mode
    fn editing_footer(&self) -> &[&str];
}

pub fn default_handle_event(key: &KeyEvent) -> Option<bool> {
    match key.code {
        // Swallow the exit key events and return "done_editing"
        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Some(false),
        _ => None,
    }
}

pub struct MutStr {
    val: String,
    orig: Option<String>,
    /// This is a valid byte offset into `val`
    cursor: usize,
}

pub enum Editable {
    ConstStr(String),
    ConstBool(bool),
    Str(MutStr),
    Bool(bool, Option<bool>),
}
impl From<&str> for Editable {
    fn from(v: &str) -> Self {
        Self::ConstStr(v.to_string())
    }
}
impl From<String> for Editable {
    fn from(v: String) -> Self {
        Self::Str(MutStr {
            cursor: v.len(),
            val: v,
            orig: None,
        })
    }
}
impl From<&bool> for Editable {
    fn from(v: &bool) -> Self {
        Self::ConstBool(*v)
    }
}
impl From<&mut bool> for Editable {
    fn from(v: &mut bool) -> Self {
        Self::Bool(*v, None)
    }
}
impl EditableWidget for Editable {
    fn as_str(&self) -> &str {
        match self {
            Self::ConstStr(s) => s.as_str(),
            Self::Str(s) => s.val.as_str(),
            Self::Bool(b, ..) | Self::ConstBool(b) => {
                if *b {
                    "true"
                } else {
                    "false"
                }
            }
        }
    }
    fn as_spans(&self, is_editing: bool) -> Vec<Span> {
        if !is_editing {
            return vec![Span::raw(self.as_str())];
        }

        let underlined = Style::default().add_modifier(Modifier::UNDERLINED);
        if let Editable::Str(s) = self {
            // Return spans with the cursor position underlined
            let mut r = Vec::with_capacity(3);
            let cursor_char = if s.cursor == s.val.len() {
                " "
            } else {
                decode_char(s.val.as_bytes(), s.cursor)
            };
            if s.cursor > 0 {
                r.push(Span::raw(&s.val[..s.cursor]));
            }
            r.push(Span::styled(cursor_char, underlined));
            if s.cursor < s.val.len() {
                r.push(Span::raw(&s.val[s.cursor + cursor_char.len()..]));
            }
            r
        } else {
            vec![Span::styled(self.as_str(), underlined)]
        }
    }

    fn handle_event(&mut self, key: &KeyEvent) -> Option<bool> {
        match self {
            Self::Bool(cur, orig) => {
                if let KeyCode::Enter = key.code {
                    if orig.is_none() {
                        *orig = Some(*cur);
                    }
                    *cur = !*cur;
                    return Some(true);
                }
            }
            Self::Str(s) => {
                let handled = match key.code {
                    KeyCode::Char(c) => {
                        if s.orig.is_none() {
                            s.orig = Some(s.val.clone());
                        }
                        s.val.insert(s.cursor, c);
                        s.cursor += c.len_utf8();
                        Some(true)
                    }
                    KeyCode::Esc => {
                        if let Some(v) = s.orig.take() {
                            s.val = v;
                            s.cursor = s.val.len();
                        }
                        Some(false)
                    }
                    KeyCode::Enter => Some(false),
                    KeyCode::Delete => {
                        if s.cursor == s.val.len() {
                            Some(true)
                        } else {
                            if s.orig.is_none() {
                                s.orig = Some(s.val.clone());
                            }
                            s.val.remove(s.cursor);
                            Some(true)
                        }
                    }
                    KeyCode::Backspace => {
                        if s.cursor == 0 {
                            Some(true)
                        } else {
                            if s.orig.is_none() {
                                s.orig = Some(s.val.clone());
                            }
                            let mut prev_offset = 0;
                            for c in s.val.char_indices() {
                                if c.0 == s.cursor {
                                    break;
                                }
                                prev_offset = c.0;
                            }
                            let c = s.val.remove(prev_offset);
                            s.cursor -= c.len_utf8();
                            Some(true)
                        }
                    }
                    KeyCode::Left => {
                        dec_val(&mut s.cursor, 1);
                        Some(true)
                    }
                    KeyCode::Right => {
                        inc_val(&mut s.cursor, 1, s.val.len() + 1);
                        Some(true)
                    }
                    KeyCode::End => {
                        s.cursor = s.val.len();
                        Some(true)
                    }
                    KeyCode::Home => {
                        s.cursor = 0;
                        Some(true)
                    }
                    _ => None,
                };
                if handled.is_some() {
                    return handled;
                }
            }
            _ => return None,
        }
        default_handle_event(key)
    }
    fn is_changed(&self) -> bool {
        match self {
            Self::ConstBool(_) | Self::ConstStr(_) => false,
            Self::Bool(cur, orig) => {
                if let Some(orig) = orig {
                    *cur != *orig
                } else {
                    false
                }
            }
            Self::Str(s) => {
                if let Some(ref orig) = s.orig {
                    s.val != orig.as_str()
                } else {
                    false
                }
            }
        }
    }

    fn is_editable(&self) -> bool {
        !matches!(self, Editable::ConstStr(_) | Editable::ConstBool(_))
    }

    fn editing_footer(&self) -> &[&str] {
        match self {
            Self::Str(_) => EDITING_STR_FOOTER,
            Self::Bool(..) => EDITING_BOOL_FOOTER,
            _ => &[],
        }
    }
}

/// Increments `orig` by `amount` without going >= `max`
pub fn inc_val(orig: &mut usize, amount: usize, max: usize) {
    orig.add_assign(amount);
    if *orig >= max {
        if max == 0 {
            *orig = 0;
        } else {
            *orig = max - 1;
        }
    }
}

/// Decrements `orig` by `amount` without going bellow 0
pub fn dec_val(orig: &mut usize, amount: usize) {
    if *orig <= amount {
        *orig = 0;
    } else {
        orig.sub_assign(amount);
    }
}

/// Decodes a single char as a &str from `&bytes[offset..]`
/// TODO : Parse the first utf8 byte to determine the length instead of retrying up to 4 times
pub fn decode_char(bytes: &[u8], offset: usize) -> &str {
    let mut cur_char = "?";
    for i in 1..5 {
        if let Ok(s) = std::str::from_utf8(&bytes[offset..offset + i]) {
            cur_char = s;
            break;
        }
    }
    cur_char
}
