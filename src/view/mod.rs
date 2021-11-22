use std::{borrow::Cow, io::Stdout};

use tui::{
    backend::CrosstermBackend,
    layout::Rect,
    style::{Color, Style},
    text::{Span, Spans},
    widgets::{Block, BorderType, Borders, Tabs},
    Frame,
};
use variant_count::VariantCount;

use crate::state::State;

pub mod prompt;
pub use prompt::*;
pub mod home;
pub use home::*;
pub mod users;
pub use users::*;

pub const HEADER_HEIGHT: u16 = 3;
pub const INFO_HEIGHT: u16 = 2;

pub const TABS: [&str; View::VARIANT_COUNT - 1] = ["Summary", "Users"];
pub const TAB_ORDER: [View; View::VARIANT_COUNT - 1] = [View::Home, View::UserList];
pub const DEFAULT_INFO: &[Cow<'static, str>] = &[
    Cow::Borrowed("[F1] Show Help"),
    Cow::Borrowed("[Esc/Q] Exit"),
    Cow::Borrowed("[Tab/Shift+Tab] Switch Tabs"),
];

#[derive(Clone, Copy, VariantCount)]
pub enum View {
    Home = 0,
    UserList,

    Prompt,
}
impl Default for View {
    fn default() -> Self {
        Self::Home
    }
}
impl View {
    pub fn draw<'b>(&'b self, state: &mut State, f: &mut Frame<CrosstermBackend<&mut Stdout>>) {
        let total_rect = f.size();
        // Split the terminal into zone we can draw to
        let (content_rect, mut info_rect, header_rect) = if state.show_help {
            let mut r = state.layout_with_info.split(total_rect);
            (r.pop().unwrap(), r.pop(), r.pop().unwrap())
        } else {
            let mut r = state.layout_no_info.split(total_rect);
            (r.pop().unwrap(), None, r.pop().unwrap())
        };

        // Write the top level tabs
        let titles = TABS.iter().map(|v| Spans::from(*v)).collect();
        let mut tabs = Tabs::new(titles)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::White))
            .divider("|");

        if let Some(v) = state.prev_view {
            tabs = tabs
                .select(v as usize)
                .highlight_style(Style::default().bg(Color::Red));
        } else {
            tabs = tabs
                .select(*self as usize)
                .highlight_style(Style::default().bg(Color::DarkGray));
        }

        f.render_widget(tabs, header_rect);

        // Draw the current tab's content
        match self {
            Self::Home => self.draw_home(state, f, content_rect, &mut info_rect),
            Self::UserList => {
                self.draw_users(&mut state.user_state, f, content_rect, &mut info_rect)
            }
            Self::Prompt => self.draw_prompt(state, f, content_rect, &mut info_rect),
        };

        // If the current tab didnt write to the info rect, write the default info
        if let Some(info_rect) = info_rect {
            Self::draw_info(DEFAULT_INFO.iter(), f, info_rect);
        }
    }

    /// Draws the provided list of strings to the info rect
    pub fn draw_info<'b, I: Iterator<Item = &'b Cow<'b, str>>>(
        vals: I,
        f: &mut Frame<CrosstermBackend<&mut Stdout>>,
        rect: Rect,
    ) {
        let mut spans = Vec::new();

        // Color anything in brackets
        for v in vals {
            let s = match v {
                Cow::Borrowed(s) => s,
                Cow::Owned(s) => s.as_str(),
            };

            if s.starts_with('[') {
                if let Some(end) = s.find(']') {
                    spans.push(Spans::from(vec![
                        Span::styled(&s[..end + 1], Style::default().fg(Color::Green)),
                        Span::raw(&s[end + 1..]),
                    ]));
                    continue;
                }
            }
            spans.push(Spans::from(Span::raw(s)));
        }

        let footer = Tabs::new(spans)
            .block(
                Block::default()
                    .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
                    .border_type(BorderType::Rounded),
            )
            .style(Style::default().fg(Color::DarkGray))
            .divider("|");
        f.render_widget(footer, rect);
    }
}
