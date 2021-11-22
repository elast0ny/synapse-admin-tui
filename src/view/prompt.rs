use std::{borrow::Cow, io::Stdout};

use crossterm::event::{KeyCode, KeyEvent};
use tui::{Frame, backend::CrosstermBackend, layout::{Alignment, Constraint, Direction, Layout, Rect}, style::{Color, Style}, text::{Span, Spans, Text}, widgets::{Cell, Paragraph, Row, Table, TableState, Wrap}};
use variant_count::VariantCount;

use crate::{
    editable::{self, Editable, EditableWidget, EvtResult},
    state::State,
};

use super::View;

pub const PROMPT_INFO: &[Cow<'static, str>] =
    &[Cow::Borrowed("[F1] Show Help"), Cow::Borrowed("[Esc] Back")];

pub const BUTTON_NAMES: [&str; PromptButton::VARIANT_COUNT] =
    ["[Yes]", "[No]", "[Ok]", "[Cancel]", "[Exit]"];

#[allow(dead_code)]
#[derive(Clone, Copy, VariantCount)]
pub enum PromptButton {
    Yes = 0,
    No,
    Ok,
    Cancel,
    Exit,
}

#[derive(Default)]
pub struct PromptInfo {
    pub top: String,
    pub bottom: String,
    pub fields: Vec<(String, Editable)>,
    pub buttons: Vec<PromptButton>,
    pub cursor: usize,
    pub action: Option<PromptButton>,

    table_state: TableState,
}
impl PromptInfo {
    pub fn clear(&mut self) {
        self.top.clear();
        self.bottom.clear();
        self.fields.clear();
        self.buttons.clear();
        self.cursor = 0;
        self.action = None;
    }
    pub fn cur_field(&mut self) -> Option<&mut Editable> {
        if self.fields.is_empty() {
            return None;
        }
        if self.cursor < self.fields.len() {
            Some(&mut unsafe { self.fields.get_unchecked_mut(self.cursor) }.1)
        } else {
            None
        }
    }
    pub fn cur_button(&self) -> Option<PromptButton> {
        if !self.fields.is_empty() && self.cursor < self.fields.len() {
            return None;
        }
        let button_idx = self.cursor - self.fields.len();
        if button_idx < self.buttons.len() {
            Some(unsafe { *self.buttons.get_unchecked(button_idx) })
        } else {
            None
        }
    }
}

pub fn handle_event_prompt(state: &mut State, view: &mut View, key: &KeyEvent) -> EvtResult {
    let s = &mut state.prompt;
    let (val, amount, max) = match key.code {
        KeyCode::Down => (&mut s.cursor, 1, s.fields.len() + s.buttons.len()),
        KeyCode::Up => (&mut s.cursor, -1, s.fields.len() + s.buttons.len()),
        KeyCode::PageDown => (&mut s.cursor, 5, s.fields.len() + s.buttons.len()),
        KeyCode::PageUp => (&mut s.cursor, -5, s.fields.len() + s.buttons.len()),
        KeyCode::Enter => {
            let action = s
                .cur_button()
                .unwrap_or(*s.buttons.get(0).unwrap_or(&PromptButton::Ok));
            state.prompt.action = Some(action);
            state.disable_prompt(view);
            if let Some(PromptButton::Exit) = state.prompt.action {
                return EvtResult::Stop;
            }
            return EvtResult::Continue;
        }
        KeyCode::Esc => {
            state.disable_prompt(view);
            state.prompt.action = Some(PromptButton::Exit);
            return EvtResult::Continue;
        }
        _ => return EvtResult::Pass,
    };

    editable::apply_offset(val, amount, max);
    s.table_state.select(Some(*val));
    EvtResult::Continue
}

impl super::View {
    pub fn draw_prompt<'b>(
        &'b self,
        state: &mut State,
        f: &mut Frame<CrosstermBackend<&mut Stdout>>,
        rect: Rect,
        info: &mut Option<Rect>,
    ) {
        let s = &mut state.prompt;

        let mut top_info = None;
        let mut bot_info = None;

        if !s.top.is_empty() {
            let txt = Text::from(s.top.as_str());
            let height = txt.height() as u16;
            let top = Paragraph::new(txt)
                .alignment(Alignment::Left)
                .style(Style::default().fg(Color::Yellow));
            top_info = Some((height, top));
        }
        if !s.bottom.is_empty() {
            let txt = Text::from(s.bottom.as_str());
            let height = txt.height() as u16;
            let top = Paragraph::new(txt)
                .alignment(Alignment::Left)
                .style(Style::default().fg(Color::Red))
                .wrap(Wrap{trim:false});
            bot_info = Some((height, top));
        }

        let table_rows = (s.fields.len() + s.buttons.len()) as u16;
        let layout = Layout::default().direction(Direction::Vertical);
        let mut constraints = Vec::new();
        if let Some(i) = &top_info {
            constraints.push(Constraint::Length(i.0));
        }
        match &bot_info {
            Some(i) => {
                if table_rows > 0 {
                    constraints.push(Constraint::Length(std::cmp::min(
                        rect.height - (top_info.as_ref().map_or(0, |v| v.0) + i.0),
                        table_rows,
                    )));
                }
                constraints.push(Constraint::Length(i.0));
            }
            None => {
                if table_rows > 0 {
                    constraints.push(Constraint::Percentage(100));
                }
            }
        }

        let mut rects = layout.constraints(constraints).split(rect);

        if let Some((_h, v)) = bot_info {
            f.render_widget(v, rects.pop().unwrap());
        }

        let highlight = Style::default().bg(Color::DarkGray);
        if table_rows > 0 {
            let mut rows = Vec::with_capacity(s.fields.len() + s.buttons.len());
            let mut max_width = 0;
            for (idx, field) in s.fields.iter().enumerate() {
                let (name, val) = if idx == s.cursor {
                    (
                        Span::styled(field.0.as_str(), highlight),
                        Spans::from(field.1.as_spans(true)),
                    )
                } else {
                    (
                        Span::raw(field.0.as_str()),
                        Spans::from(field.1.as_spans(false)),
                    )
                };
                if max_width < name.width() {
                    max_width = name.width();
                }
                rows.push(Row::new([
                    Cell::from(name),
                    Cell::from(":"),
                    Cell::from(val),
                ]));
            }
            for (idx, b) in s.buttons.iter().enumerate() {
                let button_idx = idx + s.fields.len();
                let val = if button_idx == s.cursor {
                    Span::styled(BUTTON_NAMES[*b as usize], highlight)
                } else {
                    Span::raw(BUTTON_NAMES[*b as usize])
                };
                rows.push(Row::new([
                    Cell::from(Spans::from(val)),
                    Cell::from(""),
                    Cell::from(""),
                ]));
            }

            // We have to draw two tables as there is no
            f.render_stateful_widget(
                Table::new(rows).widths(&[
                    Constraint::Length(max_width as u16),
                    Constraint::Length(1),
                    Constraint::Percentage(100),
                ]),
                rects.pop().unwrap(),
                &mut s.table_state,
            );
        }

        if let Some((_h, v)) = top_info {
            f.render_widget(v, rects.pop().unwrap());
        }

        if let Some(info_rect) = info.take() {
            let enter_help = if s.buttons.is_empty() {
                Cow::Borrowed("[Enter] Ok")
            } else {
                let first_button = BUTTON_NAMES[s.buttons[0] as usize];
                format!("[Enter] {}", &first_button[1..first_button.len() - 1]).into()
            };

            Self::draw_info(
                PROMPT_INFO.iter().chain(std::iter::once(&enter_help)),
                f,
                info_rect,
            );
        }
    }
}
