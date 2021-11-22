use std::{borrow::Cow, io::Stdout};

use crate::{backend::Backend, editable::{self, Editable, EditableWidget, EvtResult}, state::State};
use crossterm::event::{KeyCode, KeyEvent};
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
    Frame,
};

use super::View;

const USER_COLUMNS: [&str; 5] = ["ID", "Name", "Admin", "Guest", "Active"];

pub struct UserState {
    pub editing: bool,
    pub focus_x: usize,
    pub focus_y: usize,
    pub user_list: Vec<[Editable; USER_COLUMNS.len()]>,
    pub user_list_state: TableState,
}
impl Default for UserState {
    fn default() -> Self {
        Self {
            editing: false,
            focus_x: 0,
            focus_y: 0,
            user_list: Vec::with_capacity(16),
            user_list_state: TableState::default(),
        }
    }
}
impl UserState {
    pub fn cur_item(&mut self) -> Option<&mut Editable> {
        if self.user_list.is_empty() {
            return None;
        }
        if self.focus_y >= self.user_list.len() {
            self.focus_y = 0;
        }
        let row = unsafe { self.user_list.get_unchecked_mut(self.focus_y) };
        if self.focus_x >= row.len() {
            self.focus_x = 0;
        }
        Some(&mut row[self.focus_x])
    }

    pub fn editing_item(&mut self) -> Option<&mut Editable> {
        if !self.editing {
            return None;
        }
        self.cur_item()
    }
}

pub fn handle_event_users(state: &mut State, view: &mut View, key: &KeyEvent) -> EvtResult {
    let s = &mut state.user_state;
    let (val, amount, max) = match key.code {
        KeyCode::Down => (&mut s.focus_y, 1, s.user_list.len()),
        KeyCode::Up => (&mut s.focus_y, -1, s.user_list.len()),
        KeyCode::Right => (&mut s.focus_x, 1, USER_COLUMNS.len()),
        KeyCode::Left => (&mut s.focus_x, -1, USER_COLUMNS.len()),
        KeyCode::PageDown => (&mut s.focus_y, 5, s.user_list.len()),
        KeyCode::PageUp => (&mut s.focus_y, -5, s.user_list.len()),
        KeyCode::F(5) => {
            if let Backend::Synapse(b) = &mut state.backend {
                match b.list_users(0, if s.user_list.is_empty() {
                    32
                } else {
                    s.user_list.len()
                }) {
                    Ok(mut l) => {
                        s.user_list.clear();
                        for mut u in l.drain(..) {
                            s.user_list.push([
                                u.name.as_str().into(),
                                u.displayname.into(),
                                (&mut u.admin).into(),
                                (&mut u.is_guest).into(),
                                (&mut !u.deactivated).into(),
                            ]);
                        }
                        if s.user_list.is_empty() {
                            s.focus_y = 0;
                        } else {
                            s.focus_y = std::cmp::min(s.focus_y, s.user_list.len()-1);
                        }
                    },
                    Err(e) => {
                        state.prompt.clear();
                        state.prompt.bottom.push_str(e.as_str());
                        state.prompt.buttons.push(super::PromptButton::Ok);
                        state.enable_prompt(view);
                    },
                }
            }
            return EvtResult::Continue;
        }
        KeyCode::Enter => {
            if s.editing {
                s.editing = false;
            } else if let Some(i) = s.cur_item() {
                s.editing = i.is_editable();
            }
            return EvtResult::Continue;
        }
        KeyCode::Esc if s.editing => {
            s.editing = false;
            if let Some(s) = s.editing_item() {
                s.restore_orig();
            }
            return EvtResult::Continue;
        }
        _ => return EvtResult::Pass,
    };

    if s.editing {
        return EvtResult::Continue;
    }

    editable::apply_offset(val, amount, max);
    EvtResult::Continue
}

impl super::View {
    pub fn draw_users<'b>(
        &'b self,
        state: &mut UserState,
        f: &mut Frame<CrosstermBackend<&mut Stdout>>,
        rect: Rect,
        info: &mut Option<Rect>,
    ) {
        let header_cells = USER_COLUMNS
            .iter()
            .map(|v| Cell::from(*v));
        let header_row = Row::new(header_cells)
            .height(1)
            .style(Style::default().fg(Color::DarkGray));

        let mut num_changed = 0;
        let mut on_editable = false;
        let mut editing_info: &[&str] = &[];
        let mut table_spans = Vec::with_capacity(state.user_list.len());
        let mut widths = [Constraint::Length(7); USER_COLUMNS.len()];
        for y in 0..state.user_list.len() {
            let mut items = Vec::with_capacity(USER_COLUMNS.len());
            for x in 0..USER_COLUMNS.len() {
                let i = &state.user_list[y][x] as &dyn EditableWidget;
                let cur_focused = y == state.focus_y && x == state.focus_x;
                let editing_cur = state.editing && cur_focused;
                let val_changed = i.is_changed();
                let mut spans = i.as_spans(editing_cur);
                if editing_cur && info.is_some() {
                    editing_info = i.editing_footer();
                }
                // Color any changed value
                if val_changed {
                    num_changed += 1;
                    for s in spans.iter_mut() {
                        if val_changed {
                            s.style = s.style.fg(Color::Yellow);
                        }
                    }
                }
                let mut width_padding = 2;
                // Style the current focused item
                if cur_focused {
                    width_padding = 0;
                    state.user_list_state.select(Some(state.focus_y));
                    let s = if i.is_editable() {
                        on_editable = true;
                        if state.editing {
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default()
                                .fg(Color::Green)
                                .add_modifier(Modifier::BOLD)
                        }
                    } else {
                        Style::default().fg(Color::Black)
                    };
                    spans.insert(0, Span::styled("[", s));
                    spans.push(Span::styled("]", s));
                }
                let s = Spans::from(spans);
                match &mut widths[x] {
                    Constraint::Length(v) => {
                        if s.width() as u16 + width_padding > *v {
                            *v = s.width() as u16 + width_padding;
                        }
                    }
                    _=>unreachable!(),
                };
                items.push(Spans::from(s));
            }
            table_spans.push(Row::new(items));
        }

        let user_table = Table::new(table_spans)
            .block(Block::default().borders(Borders::LEFT | Borders::RIGHT))
            .header(header_row)
            .highlight_style(Style::default().bg(Color::DarkGray))
            .widths(&widths);
        f.render_stateful_widget(user_table, rect, &mut state.user_list_state);

        if let Some(info_rect) = info.take() {
            let mut info_vals: Vec<Cow<str>> = Vec::new();
            if state.editing {
                info_vals.extend(["[EDITING]"].iter().map(|v| Into::into(*v)));
                info_vals.extend(editing_info.iter().map(|v| Into::into(*v)));
            } else {
                info_vals.push("[F5] Refresh".into());
                if num_changed > 0 {
                    info_vals.push(format!("[F2] Apply ({})", num_changed).into());
                }
                if on_editable {
                    info_vals.push("[Enter] Edit".into());
                }
            }
            Self::draw_info(info_vals.iter(), f, info_rect);
        }
    }
}
