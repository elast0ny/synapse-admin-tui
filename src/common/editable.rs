use std::ops::Deref;

use crossterm::event::{KeyCode, KeyEvent};
use tui::{
    style::{Color, Modifier, Style},
    text::Span,
};

use super::*;

pub trait EditableWidget {
    /// Returns the contents of the widget as a string
    fn as_str(&self) -> &str;
    /// Returns the contents of the widget as spans
    fn as_spans<'b>(&'b self, is_editing: bool) -> Vec<Span<'b>>;
    /// Key events forwarded from the main loop when editing the widget
    fn handle_event(&mut self, key: &KeyEvent) -> HandleRes;
    /// If the contents have been modified
    fn is_changed(&self) -> bool;
    /// Whether this widget can be edited
    fn is_editable(&self) -> bool;
    fn restore_orig(&mut self);
    fn forget_orig(&mut self);
}

pub struct MutStr {
    cur: String,
    orig: Option<String>,
    cursor: usize,
}
impl MutStr {
    /// Saves the current value if nothing is already saved
    fn save_cur(&mut self) -> &mut String {
        if self.orig.is_none() {
            self.orig = Some(self.cur.clone());
        };
        &mut self.cur
    }
}

pub enum Editable {
    ConstStr(String),
    ConstBool(bool),
    Str(MutStr),
    Bool(bool, Option<bool>),
}
impl EditableWidget for Editable {
    fn as_str(&self) -> &str {
        match self {
            Self::ConstStr(s) => s,
            Self::Str(s) => s.cur.deref(),
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
            let cur = s.cur.deref();
            // Return spans with the cursor position underlined
            let mut r = Vec::with_capacity(3);
            if s.cursor > 0 {
                r.push(Span::raw(&cur[..s.cursor]));
            }
            if s.cursor < cur.len() {
                let cursor_char = decode_char(cur.as_bytes(), s.cursor);
                r.push(Span::styled(cursor_char, underlined));
                r.push(Span::raw(&cur[s.cursor + cursor_char.len()..]));
            } else {
                r.push(Span::styled(
                    " ",
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::UNDERLINED),
                ));
            }
            r
        } else {
            vec![Span::styled(self.as_str(), underlined)]
        }
    }

    fn handle_event(&mut self, key: &KeyEvent) -> HandleRes {
        let r = match self {
            Self::Bool(cur, orig) => {
                if let KeyCode::Enter = key.code {
                    if orig.is_none() {
                        *orig = Some(*cur);
                    }
                    *cur = !*cur;
                    HandleRes::ReDraw
                } else {
                    HandleRes::Ignored
                }
            }
            Self::Str(s) => match key.code {
                KeyCode::Char(c) => {
                    let cursor = s.cursor;
                    let mut_cur = s.save_cur();
                    mut_cur.insert(cursor, c);
                    s.cursor += c.len_utf8();
                    HandleRes::ReDraw
                }
                KeyCode::Delete => {
                    if s.cursor < s.cur.deref().len() {
                        let cursor = s.cursor;
                        let mut_cur = s.save_cur();
                        mut_cur.remove(cursor);
                        HandleRes::ReDraw
                    } else {
                        HandleRes::Handled
                    }
                }
                KeyCode::Backspace => {
                    if s.cursor > 0 {
                        let cursor = s.cursor;
                        let mut_cur = s.save_cur();
                        let mut prev_offset = 0;
                        for c in mut_cur.char_indices() {
                            if c.0 == cursor {
                                break;
                            }
                            prev_offset = c.0;
                        }
                        let c = mut_cur.remove(prev_offset);
                        s.cursor -= c.len_utf8();
                        HandleRes::ReDraw
                    } else {
                        HandleRes::Handled
                    }
                }
                KeyCode::Left => {
                    if s.cursor > 0 {
                        dec_val(&mut s.cursor, 1);
                        HandleRes::ReDraw
                    } else {
                        HandleRes::Handled
                    }
                }
                KeyCode::Right => {
                    let old = s.cursor;
                    inc_val(&mut s.cursor, 1, s.cur.deref().len() + 1);
                    if old != s.cursor {
                        HandleRes::ReDraw
                    } else {
                        HandleRes::Handled
                    }
                }
                KeyCode::End => {
                    if s.cursor != s.cur.deref().len() {
                        s.cursor = s.cur.deref().len();
                        HandleRes::ReDraw
                    } else {
                        HandleRes::Handled
                    }
                }
                KeyCode::Home => {
                    if s.cursor != 0 {
                        s.cursor = 0;
                        HandleRes::ReDraw
                    } else {
                        HandleRes::Handled
                    }
                }
                _ => HandleRes::Ignored,
            },
            _ => HandleRes::Ignored,
        };
        r
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
                if let Some(orig) = s.orig.as_deref() {
                    s.cur != orig
                } else {
                    false
                }
            }
        }
    }

    fn is_editable(&self) -> bool {
        !matches!(self, Editable::ConstStr(_) | Editable::ConstBool(_))
    }

    fn restore_orig(&mut self) {
        match self {
            Self::Str(s) => {
                s.cur.clear();
                if let Some(orig) = s.orig.as_deref() {
                    s.cur.push_str(orig);
                }
            }
            Self::Bool(cur, orig) => {
                if let Some(v) = orig.take() {
                    *cur = v;
                }
            }
            _ => {}
        }
    }
    fn forget_orig(&mut self) {
        match self {
            Self::Str(s) => {
                s.orig.take();
            }
            Self::Bool(_cur, orig) => {
                orig.take();
            }
            _ => {}
        }
    }
}

impl Editable {
    /// Constructs a read only string
    pub fn ro_string(s: &str) -> Self {
        Self::ConstStr(s.to_string())
    }
    /// Constructs a read only bool
    pub fn ro_bool(b: bool) -> Self {
        Self::ConstBool(b)
    }

    /// Constructs an editable string
    pub fn string(s: &str) -> Self {
        Self::Str(MutStr {
            cur: s.to_string(),
            orig: None,
            cursor: s.len(),
        })
    }

    /// Constructs an editable bool
    pub fn bool(b: bool) -> Self {
        Self::Bool(b, None)
    }
}
