use crossterm::event::{Event, KeyCode};
use tui::{
    layout::Constraint,
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
};

use crate::{
    backend::Synapse,
    common::{
        apply_offset,
        editable::{Editable, EditableWidget},
        prompt::Prompt,
        HandleRes, ViewImpl,
    },
    state::State,
};

enum CurPrompt {
    None,
    Notice,
    SaveChanges,
}
impl Default for CurPrompt {
    fn default() -> Self {
        Self::None
    }
}

enum SyncState {
    Some,
    Max,
}
impl Default for SyncState {
    fn default() -> Self {
        Self::Some
    }
}

const USER_COLUMNS: [&str; 5] = ["ID", "Name", "Admin", "Guest", "Active"];

#[derive(Default)]
pub struct UsersView {
    cur_prompt: CurPrompt,
    prompt: Prompt,
    sync_state: SyncState,
    editing: bool,
    focus_x: usize,
    focus_y: usize,
    user_list: Vec<[Editable; USER_COLUMNS.len()]>,
    user_list_state: TableState,
}

impl ViewImpl<State> for UsersView {
    fn title(&self) -> &'static str {
        "Users"
    }
    fn enter_view(&mut self, state: &mut State) {
        if let Err(e) = self.load_next_chunk(&mut state.backend) {
            self.prompt.clear();
            self.prompt.error.push_str(e.as_str());
            self.prompt.true_button.push_str("Ok");
            self.cur_prompt = CurPrompt::Notice;
        }
    }
    fn draw_view(
        &mut self,
        frame: &mut tui::Frame<tui::backend::CrosstermBackend<&mut std::io::Stdout>>,
        rect: tui::layout::Rect,
        _state: &mut State,
    ) {
        if !matches!(self.cur_prompt, CurPrompt::None) {
            self.prompt.draw_view(frame, rect, &mut ());
            return;
        }

        let header_cells = USER_COLUMNS.iter().map(|v| Cell::from(*v));
        let header_row = Row::new(header_cells)
            .height(1)
            .style(Style::default().fg(Color::DarkGray));

        let mut table_spans = Vec::with_capacity(self.user_list.len());
        let mut widths = [Constraint::Length(7); USER_COLUMNS.len()];
        for y in 0..self.user_list.len() {
            let mut items = Vec::with_capacity(USER_COLUMNS.len());
            for x in 0..USER_COLUMNS.len() {
                let i = &self.user_list[y][x];
                let cur_focused = y == self.focus_y && x == self.focus_x;
                let editing_cur = self.editing && cur_focused;
                let val_changed = i.is_changed();
                let mut spans = i.as_spans(editing_cur);
                // Color any changed value
                if val_changed {
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
                    self.user_list_state.select(Some(self.focus_y));
                    let s = if i.is_editable() {
                        if self.editing {
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
                    _ => unreachable!(),
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
        frame.render_stateful_widget(user_table, rect, &mut self.user_list_state);
    }

    fn handle_event(&mut self, event: &Event, state: &mut State) -> HandleRes {
        match self.cur_prompt {
            CurPrompt::Notice => {
                if let HandleRes::Exit(_) = self.prompt.handle_event(event, &mut ()) {
                    self.cur_prompt = CurPrompt::None;
                    return HandleRes::ReDraw;
                }
            }
            CurPrompt::SaveChanges => {
                if let HandleRes::Exit(_) = self.prompt.handle_event(event, &mut ()) {
                    self.cur_prompt = CurPrompt::None;
                    return HandleRes::ReDraw;
                }
            }
            _ => {}
        };

        let key = match event {
            Event::Key(k) => k,
            _ => return HandleRes::Ignored,
        };

        // Pass keystrokes to editable widget
        if let Some(i) = self.editing_item() {
            let r = i.handle_event(&key);
            if matches!(r, HandleRes::ReDraw | HandleRes::Handled) {
                return r;
            }
        }

        let (val, amount, max) = match key.code {
            KeyCode::Down => (&mut self.focus_y, 1, self.user_list.len()),
            KeyCode::Up => (&mut self.focus_y, -1, self.user_list.len()),
            KeyCode::Right => (&mut self.focus_x, 1, USER_COLUMNS.len()),
            KeyCode::Left => (&mut self.focus_x, -1, USER_COLUMNS.len()),
            KeyCode::PageDown => (&mut self.focus_y, 5, self.user_list.len()),
            KeyCode::PageUp => (&mut self.focus_y, -5, self.user_list.len()),
            KeyCode::F(5) => {
                self.focus_y = 0;
                self.user_list.clear();
                if let Err(e) = self.load_next_chunk(&mut state.backend) {
                    self.prompt.clear();
                    self.prompt.error.push_str(e.as_str());
                    self.prompt.true_button.push_str("Ok");
                    self.cur_prompt = CurPrompt::Notice;
                }
                return HandleRes::ReDraw;
            }
            KeyCode::Enter => {
                if self.editing {
                    self.editing = false;
                } else if let Some(i) = self.cur_item() {
                    if !i.is_editable() {
                        return HandleRes::Handled;
                    } else {
                        self.editing = true;
                    }
                }
                return HandleRes::ReDraw;
            }
            KeyCode::Esc if self.editing => {
                self.editing = false;
                if let Some(s) = self.editing_item() {
                    s.restore_orig();
                }
                return HandleRes::ReDraw;
            }
            _ => return HandleRes::Ignored,
        };

        let old = *val;
        apply_offset(val, amount, max);
        let new = *val;
        // Disable edit mode if we landed on non editable field
        if let Some(i) = self.cur_item() {
            if !i.is_editable() && self.editing {
                self.editing = false;
            }
        }
        if old != new {
            return HandleRes::ReDraw;
        } else {
            return HandleRes::Handled;
        }
    }
}

impl UsersView {
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

    fn load_next_chunk(&mut self, synapse: &mut Synapse) -> Result<usize, String> {
        if let SyncState::Max = self.sync_state {
            return Ok(0);
        }
        let mut l = synapse.list_users(self.user_list.len(), 32)?;
        let num_received = l.len();
        for u in l.drain(..) {
            self.user_list.push([
                Editable::ro_string(u.name.as_str()),
                Editable::string(u.displayname.as_str()),
                Editable::bool(u.admin),
                Editable::bool(u.is_guest),
                Editable::bool(!u.deactivated),
            ]);
        }
        // We got less than what we queried for, we hit the end
        if num_received < 32 {
            self.sync_state = SyncState::Max;
        } else {
            self.sync_state = SyncState::Some;
        }
        Ok(num_received)
    }
}
