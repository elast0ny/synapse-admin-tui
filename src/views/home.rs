use crate::{
    common::{HandleRes, ViewImpl},
    state::State,
};

#[derive(Default)]
pub struct HomeView {}

impl ViewImpl<State> for HomeView {
    fn title(&self) -> &'static str {
        "Summary"
    }

    fn draw_view(
        &mut self,
        _frame: &mut tui::Frame<tui::backend::CrosstermBackend<&mut std::io::Stdout>>,
        _rect: tui::layout::Rect,
        _state: &mut State,
    ) {
    }

    fn handle_event(&mut self, _event: &crossterm::event::Event, _state: &mut State) -> HandleRes {
        HandleRes::Ignored
    }
}
