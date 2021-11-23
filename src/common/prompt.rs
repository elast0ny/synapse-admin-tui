use std::io::Stdout;

use crossterm::event::{Event, KeyCode};
use tui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Cell, Paragraph, Row, Table, TableState, Tabs, Wrap},
    Frame,
};

use crate::editable::*;

use super::*;

#[derive(Default)]
pub struct Prompt {
    pub msg: String,
    pub error: String,
    pub fields: Vec<(String, Editable)>,
    pub true_button: String,
    pub false_button: String,
    pub cursor: usize,
}

impl Prompt {
    pub fn new(
        msg: Option<&str>,
        error: Option<&str>,
        fields: Option<Vec<(String, Editable)>>,
        true_button: Option<&str>,
        false_button: Option<&str>,
        cursor: usize,
    ) -> Self {
        Self {
            msg: msg.unwrap_or("").to_string(),
            error: error.unwrap_or("").to_string(),
            fields: fields.unwrap_or(Vec::with_capacity(4)),
            true_button: true_button.unwrap_or("").to_string(),
            false_button: false_button.unwrap_or("").to_string(),
            cursor,
        }
    }

    pub fn clear(&mut self) {
        self.msg.clear();
        self.error.clear();
        self.fields.clear();
        self.true_button.clear();
        self.false_button.clear();
        self.cursor = 0;
    }
}

impl ViewImpl<()> for Prompt {
    fn title(&self) -> &'static str {
        "Prompt"
    }
    /// Draw the current view using the provided ViewInfo
    fn draw_view(
        &mut self,
        frame: &mut Frame<CrosstermBackend<&mut Stdout>>,
        rect: Rect,
        _state: &mut (),
    ) {
        let mut constraints = Vec::with_capacity(4);
        let mut table_state = TableState::default();
        let mut rows = Vec::with_capacity(self.fields.len());
        let mut buttons: Vec<Spans> = Vec::with_capacity(2);
        let mut max_field_title = 0;
        let mut space_taken = 0;
        let mut num_fields = 0;

        // Calculate sizes and such for UI elements

        // Prompt message
        let mut msg_widget = None;
        if !self.msg.is_empty() {
            let txt = Text::from(self.msg.as_str());
            let height = txt.height() as u16;
            space_taken += height;
            constraints.push(Constraint::Length(height));
            let w = Paragraph::new(txt)
                .alignment(Alignment::Left)
                .style(Style::default().fg(Color::Yellow));
            msg_widget = Some(w);
        }

        // Error Message
        let mut err_widget = None;
        if !self.error.is_empty() {
            let txt = Text::from(self.error.as_str());
            let height = txt.height() as u16;
            space_taken += height;
            let w = Paragraph::new(txt)
                .alignment(Alignment::Left)
                .style(Style::default().fg(Color::Red))
                .wrap(Wrap { trim: false });
            err_widget = Some((height, w));
        }

        // Buttons
        if !self.true_button.is_empty() {
            buttons.push(
                vec![
                    Span::styled("[Enter] ", Style::default().fg(Color::Green)),
                    Span::raw(self.true_button.as_str()),
                ]
                .into(),
            );
        }
        if !self.false_button.is_empty() {
            buttons.push(
                vec![
                    Span::styled("[Esc] ", Style::default().fg(Color::Green)),
                    Span::raw(self.false_button.as_str()),
                ]
                .into(),
            );
        }
        if !self.true_button.is_empty() || !self.false_button.is_empty() {
            space_taken += 1;
        }

        // Editable options in prompt
        if !self.fields.is_empty() {
            // how much space we have to draw the fields
            num_fields = self.fields.len();
            let field_space = std::cmp::min(
                if rect.height > space_taken {
                    rect.height - space_taken
                } else {
                    0
                },
                num_fields as u16,
            );
            constraints.push(Constraint::Length(field_space));

            // Set the selected editable field
            if self.cursor < rows.len() {
                table_state.select(Some(self.cursor));
            }

            for (idx, r) in self.fields.iter().enumerate() {
                let (name, val) = (
                    Span::raw(r.0.as_str()),
                    Spans::from(r.1.as_spans(idx == self.cursor)),
                );
                rows.push(Row::new([
                    Cell::from(name),
                    Cell::from(":"),
                    Cell::from(val),
                ]));

                if r.0.len() > max_field_title {
                    max_field_title = r.0.len();
                }
            }
        }

        if let Some(w) = &err_widget {
            constraints.push(Constraint::Length(w.0));
        }
        constraints.push(Constraint::Length(1));

        // Split dst rect into our calculated zones
        let mut rects = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints.clone())
            .split(rect);

        // Draw buttons
        let mut buttons_w = Tabs::new(buttons.clone())
            .style(Style::default().fg(Color::DarkGray))
            .divider("|");
        if self.cursor >= num_fields {
            buttons_w = buttons_w.style(Style::default().add_modifier(Modifier::BOLD));
        }
        frame.render_widget(buttons_w, rects.pop().unwrap());

        // Draw error message
        if let Some((_h, w)) = &err_widget {
            frame.render_widget(w.clone(), rects.pop().unwrap());
        }

        // Draw editable fields
        if !rows.is_empty() {
            frame.render_stateful_widget(
                Table::new(rows)
                    .widths(&[
                        Constraint::Length(max_field_title as u16),
                        Constraint::Length(1),
                        Constraint::Percentage(100),
                    ])
                    .highlight_style(Style::default().add_modifier(Modifier::BOLD)),
                rects.pop().unwrap(),
                &mut table_state,
            );
        }

        // Draw message
        if let Some(w) = &msg_widget {
            frame.render_widget(w.clone(), rects.pop().unwrap());
        }
    }

    /// Performs the required logic based on incoming events
    fn handle_event(&mut self, event: &Event, _: &mut ()) -> HandleRes {
        // Handle key events
        if let Event::Key(key) = event {
            // If the current focus is an editable field
            // forward the event to it
            if self.cursor < self.fields.len() {
                let edit_widget = &mut self.fields[self.cursor].1;
                let r = edit_widget.handle_event(&key);
                if matches!(r, HandleRes::ReDraw | HandleRes::Handled) {
                    return r;
                }
            }

            // Handle any prompt specific keys
            let (val, amount, max) = match key.code {
                KeyCode::Down => (&mut self.cursor, 1, self.fields.len() + 1),
                KeyCode::Up => (&mut self.cursor, -1, self.fields.len() + 1),
                KeyCode::PageDown => (&mut self.cursor, 5, self.fields.len() + 1),
                KeyCode::PageUp => (&mut self.cursor, -5, self.fields.len() + 1),
                KeyCode::Enter => return HandleRes::Exit(true),
                KeyCode::Esc => return HandleRes::Exit(false),
                _ => return HandleRes::Ignored,
            };
            let old = *val;
            apply_offset(val, amount, max);
            if old != *val {
                return HandleRes::ReDraw;
            } else {
                return HandleRes::Handled;
            }
        }
        HandleRes::Ignored
    }
}
