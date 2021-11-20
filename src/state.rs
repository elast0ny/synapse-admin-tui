use tui::{layout::{Constraint, Direction, Layout}, widgets::TableState};

use crate::{editable::{self, Editable, EditableWidget}, view::{HEADER_HEIGHT, INFO_HEIGHT, TAB_ORDER, View}};

pub const USER_FIELDS: [&str; 3] = ["Name", "Email", "IsAdmin"];

pub struct State {
    pub editing: bool,
    pub show_help: bool,
    pub cur_focus: (usize, usize),

    pub layout_with_info: Layout,
    pub layout_no_info: Layout,
    
    pub user_list_headers: Vec<String>,
    pub user_list: Vec<Vec<Editable>>,
    pub user_list_state: TableState,
}
impl Default for State {
    fn default() -> Self {
        Self {
            editing: false,
            cur_focus: (0, 0),
            show_help: true,

            layout_with_info: Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(HEADER_HEIGHT),
                    Constraint::Min(INFO_HEIGHT),
                    Constraint::Percentage(100),
                ]),
            layout_no_info: Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(HEADER_HEIGHT),
                    Constraint::Percentage(100),
                ]),

            user_list_headers: USER_FIELDS.iter().map(|v|v.to_string()).collect(),
            user_list: Vec::with_capacity(16),
            user_list_state: TableState::default(),
        }
    }
}

impl State {
    pub fn toggle_edit(&mut self, view: &View) {
        if !self.editing {
            // Only set editing to true if we are focusing an editable widget
            match self.cur_focused(view) {
                Some(h) if h.is_editable() => self.editing = true,
                _ => {}
            };
            return;
        }
        self.editing = false;
    }
    pub fn cur_focused<'b>(&'b mut self, view: &View) -> Option<&'b mut dyn EditableWidget> {
        match view {
            View::UserList => {
                if self.user_list.is_empty() {
                    return None;
                }
                if self.cur_focus.0 >= self.user_list.len() {
                    self.cur_focus.0 = 0;
                }
                let row = unsafe { self.user_list.get_unchecked_mut(self.cur_focus.0) };
                if self.cur_focus.1 >= row.len() {
                    self.cur_focus.1 = 0;
                }
                Some(&mut row[self.cur_focus.1])
            }
            _ => None,
        }
    }

    pub fn inc_row(&mut self, view: &View, amount: usize) {
        match view {
            View::UserList => {
                editable::inc_val(&mut self.cur_focus.0, amount, self.user_list.len());
                self.user_list_state.select(Some(self.cur_focus.0));
            }
            _ => {
                self.cur_focus.0 = 0;
                return;
            }
        };

        self.editing = false;
    }

    pub fn prev_row(&mut self, view: &View, amount: usize) {
        match view {
            View::UserList => {
                editable::dec_val(&mut self.cur_focus.0, amount);
                self.user_list_state.select(Some(self.cur_focus.0));
            }
            _ => {
                self.cur_focus.0 = 0;
                return;
            }
        };
        self.editing = false;
    }

    pub fn inc_col(&mut self, view: &View, amount: usize) {
        let max = match view {
            View::UserList => self.user_list_headers.len(),
            _ => {
                self.cur_focus.1 = 0;
                return;
            }
        };
        editable::inc_val(&mut self.cur_focus.1, amount, max);
        self.editing = false;
    }

    pub fn prev_col(&mut self, _view: &View, amount: usize) {
        editable::dec_val(&mut self.cur_focus.1, amount);
        self.editing = false;
    }

    pub fn next_view(&mut self, view: &mut View) {
        let mut v = *view as usize;
        editable::inc_val(&mut v, 1, TAB_ORDER.len());
        *view = TAB_ORDER[v];
    }

    pub fn prev_view(&mut self, view: &mut View) {
        let mut v = *view as usize;
        editable::dec_val(&mut v, 1);
        *view = TAB_ORDER[v];
    }
}
