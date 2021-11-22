use std::io::Stdout;

use tui::{backend::CrosstermBackend, layout::Rect, Frame};

use crate::state::State;

impl super::View {
    pub fn draw_home<'b>(
        &'b self,
        _state: &mut State,
        _f: &mut Frame<CrosstermBackend<&mut Stdout>>,
        _rect: Rect,
        _info: &mut Option<Rect>,
    ) {
    }
}
