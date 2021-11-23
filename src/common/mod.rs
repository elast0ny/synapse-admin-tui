use std::{
    io::Stdout,
    ops::{AddAssign, SubAssign},
};

use crossterm::event::Event;
use tui::{backend::CrosstermBackend, layout::Rect, Frame};

pub enum HandleRes {
    /// The view wants to bail out
    Exit(bool),
    /// The event was handled and nothing changed
    Handled,
    /// The event was handled and the ui needs redrawing
    ReDraw,
    /// The event was not handled
    Ignored,
}

pub trait ViewImpl<S> {
    /// The display value of this view
    fn title(&self) -> &'static str;

    /// Called when we're about to enter a view
    fn enter_view(&mut self, _state: &mut S) {}

    /// Draw into the provided rect
    fn draw_view(
        &mut self,
        _frame: &mut Frame<CrosstermBackend<&mut Stdout>>,
        _rect: Rect,
        _state: &mut S,
    ) {
    }

    /// Performs the required logic based on incoming events
    fn handle_event(&mut self, _event: &Event, _state: &mut S) -> HandleRes {
        HandleRes::Ignored
    }

    /// Called when we're about to leave the view
    fn leave_view(&mut self, _state: &mut S) -> HandleRes {
        HandleRes::Ignored
    }
}

pub mod editable;
pub mod prompt;

/// Increments `orig` by `amount` without going >= `max`
pub fn inc_val(orig: &mut usize, amount: usize, max: usize) -> usize {
    orig.add_assign(amount);
    if *orig >= max {
        if max == 0 {
            *orig = 0;
            return amount;
        } else {
            let overflow = *orig - (max - 1);
            *orig = max - 1;
            return overflow;
        }
    }
    return 0;
}

/// Decrements `orig` by `amount` without going bellow 0
pub fn dec_val(orig: &mut usize, amount: usize) {
    if *orig <= amount {
        *orig = 0;
    } else {
        orig.sub_assign(amount);
    }
}

pub fn apply_offset(orig: &mut usize, offset: isize, max: usize) -> usize {
    if offset > 0 {
        return inc_val(orig, offset.abs() as usize, max);
    }
    dec_val(orig, offset.abs() as usize);
    0
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
