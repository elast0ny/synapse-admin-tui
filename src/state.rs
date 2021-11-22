use crossterm::event::KeyEvent;
use tui::layout::{Constraint, Direction, Layout};

use crate::{
    backend::Backend,
    editable::{self, EvtResult},
    view::{PromptInfo, UserState, View, HEADER_HEIGHT, INFO_HEIGHT, TAB_ORDER},
};

pub struct State {
    pub prev_view: Option<View>,
    pub show_help: bool,
    pub cur_focus: (usize, usize),
    pub backend: Backend,

    pub layout_with_info: Layout,
    pub layout_no_info: Layout,

    pub prompt: PromptInfo,
    pub user_state: UserState,
}

pub fn handle_event_passthrough(
    _state: &mut State,
    _view: &mut View,
    _key: &KeyEvent,
) -> EvtResult {
    EvtResult::Pass
}

impl State {
    pub fn new(backend: Backend) -> Self {
        Self {
            prev_view: None,
            cur_focus: (0, 0),
            show_help: true,
            backend,
            layout_with_info: Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(HEADER_HEIGHT),
                    Constraint::Min(INFO_HEIGHT),
                    Constraint::Percentage(100),
                ]),
            layout_no_info: Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(HEADER_HEIGHT), Constraint::Percentage(100)]),

            prompt: Default::default(),
            user_state: Default::default(),
        }
    }

    pub fn check_backend_prompt(&mut self, view: &mut View) {
        // Theres already a prompt in progress
        if self.prev_view.is_some() {
            return;
        }

        let set_prompt = match &mut self.backend {
            Backend::Synapse(s) => s.update_prompt(&mut self.prompt),
        };

        if set_prompt {
            self.enable_prompt(view);
        } else {
            self.prev_view = None;
            self.prompt.action = None;
        }
    }

    pub fn enable_prompt(&mut self, view: &mut View) {
        self.prompt.action = None;
        self.prev_view = Some(*view);
        *view = View::Prompt;
    }
    pub fn disable_prompt(&mut self, view: &mut View) {
        if let Some(prev) = self.prev_view.take() {
            *view = prev;
        }
    }

    pub fn next_view(&mut self, view: &mut View) {
        if matches!(view, View::Prompt) {
            return;
        }
        let mut v = *view as usize;
        editable::inc_val(&mut v, 1, TAB_ORDER.len());
        self.cur_focus = (0, 0);
        *view = TAB_ORDER[v];
    }

    pub fn prev_view(&mut self, view: &mut View) {
        if matches!(view, View::Prompt) {
            return;
        }
        let mut v = *view as usize;
        editable::dec_val(&mut v, 1);
        self.cur_focus = (0, 0);
        *view = TAB_ORDER[v];
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }
}
