use std::io::Stdout;

use crossterm::event::{Event, KeyCode};
use tui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Span, Spans, Text},
    widgets::{Block, BorderType, Borders, Paragraph, Tabs},
    Frame,
};

use crate::{
    backend::{BackendImpl, Synapse},
    common::{apply_offset, dec_val, inc_val, prompt::Prompt, HandleRes, ViewImpl},
};

pub struct State {
    show_help: bool,
    tabs: Vec<&'static str>,
    cur_tab: usize,

    backend_prompt: bool,
    prompt: Prompt,

    layout_with_info: Layout,
    layout_no_info: Layout,

    pub backend: Synapse,
}

impl State {
    pub fn from_views<'a, I>(views: I, backend: Synapse) -> Self
    where
        I: Iterator<Item = &'a mut dyn ViewImpl<Self>>,
    {
        Self {
            show_help: true,
            tabs: views.map(|v| v.title()).collect(),
            cur_tab: 0,

            backend_prompt: false,
            prompt: Prompt::default(),

            layout_with_info: Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(3),
                    Constraint::Min(2),
                    Constraint::Percentage(100),
                ]),
            layout_no_info: Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(3), Constraint::Percentage(100)]),
            backend,
        }
    }

    /// Draws the base layout and returns the content rect
    pub fn draw_base(
        &mut self,
        frame: &mut Frame<CrosstermBackend<&mut Stdout>>,
        rect: Rect,
    ) -> Option<Rect> {
        let mut rects = if self.show_help {
            &self.layout_with_info
        } else {
            &self.layout_no_info
        }
        .split(rect);

        let content_rect = rects.pop().unwrap();

        if self.show_help {
            let gray = Style::default().fg(Color::DarkGray);
            let green = Style::default().fg(Color::Green);
            let rect = rects.pop().unwrap();
            let text = Text::from(vec![Spans::from(vec![
                Span::styled("[", gray),
                Span::styled("F1", green),
                Span::styled("] Toggle help  | [", gray),
                Span::styled("Esc/Q", green),
                Span::styled("] Back | [", gray),
                Span::styled("Tab/Shift+Tab", green),
                Span::styled("] Navigate tabs", gray),
            ])]);
            frame.render_widget(
                Paragraph::new(text)
                    .block(
                        Block::default()
                            .borders(Borders::LEFT | Borders::BOTTOM | Borders::RIGHT)
                            .border_style(gray)
                            .border_type(BorderType::Rounded),
                    )
                    .alignment(Alignment::Center),
                rect,
            );
        }

        // Draw the tabs
        let rect = rects.pop().unwrap();
        let titles = self.tabs.iter().map(|v| Spans::from(*v)).collect();
        let tabs = Tabs::new(titles)
            .select(self.cur_tab)
            .highlight_style(Style::default().bg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::White))
            .divider("|");
        frame.render_widget(tabs, rect);

        if self.backend_prompt || self.backend.set_prompt(&mut self.prompt) {
            self.backend_prompt = true;
            self.prompt.draw_view(frame, content_rect, &mut ());
            return None;
        }

        Some(content_rect)
    }

    pub fn handle_event_pre(&mut self, event: &Event) -> HandleRes {
        if self.backend_prompt {
            let mut r = self.prompt.handle_event(event, &mut ());
            if self.backend.prompt_done(&mut self.prompt, &mut r) {
                self.backend_prompt = false;
                return r;
            }

            if !matches!(r, HandleRes::Ignored) {
                return r;
            }
        }

        if let Event::Key(key) = event {
            let (val, amount, max) = match key.code {
                KeyCode::Tab => (&mut self.cur_tab, 1, self.tabs.len()),
                KeyCode::BackTab => (&mut self.cur_tab, -1, self.tabs.len()),
                KeyCode::F(1) => {
                    self.show_help = !self.show_help;
                    return HandleRes::ReDraw;
                }
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

    pub fn handle_event_last(&mut self, event: &Event) -> HandleRes {
        if let Event::Key(key) = event {
            // If no one handled escape, exit
            if matches!(
                key.code,
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc
            ) {
                return HandleRes::Exit(true);
            }
        }
        HandleRes::Ignored
    }

    pub fn next_tab(&mut self) {
        inc_val(&mut self.cur_tab, 1, self.tabs.len());
    }

    pub fn prev_tab(&mut self) {
        dec_val(&mut self.cur_tab, 1)
    }

    pub fn cur_tab(&self) -> usize {
        self.cur_tab
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }
}
