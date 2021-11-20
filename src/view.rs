use std::{borrow::Cow, io::Stdout};

use tui::{Frame, backend::CrosstermBackend, layout::{Constraint, Rect}, style::{Color, Modifier, Style}, text::{Span, Spans}, widgets::{Block, BorderType, Borders, Cell, Row, Table, Tabs}};
use variant_count::VariantCount;

use crate::{
    editable::EditableWidget,
    state::State,
};

pub const HEADER_HEIGHT: u16 = 3;
pub const INFO_HEIGHT: u16 = 2;

pub const TABS: [&str; View::VARIANT_COUNT] = ["Summary", "Users"];
pub const TAB_ORDER: [View; View::VARIANT_COUNT] = [View::Home, View::UserList];
pub const DEFAULT_INFO: &[&str] = &["[F1] Show Help", "[Esc/Q] Exit", "[Tab/Shift+Tab] Switch Tabs"];
pub const TABLE_INFO: &[&str] = &["[ArrowKey] Switch cell"];

#[derive(Clone, Copy, VariantCount)]
pub enum View {
    Home = 0,
    UserList,
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
        let tabs = Tabs::new(titles)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::White))
            .select(*self as usize)
            .highlight_style(Style::default().bg(Color::DarkGray))
            .divider("|");
        f.render_widget(tabs, header_rect);

        // Draw the current tab's content
        match self {
            Self::Home => self.draw_home(state, f, content_rect, &mut info_rect),
            Self::UserList => self.draw_userlist(state, f, content_rect, &mut info_rect),
        };

        // If the current tab didnt write to the info rect, write the default info
        if let Some(info_rect) = info_rect {
            Self::draw_info(
                DEFAULT_INFO.iter().map(|v| Cow::Borrowed(*v)).collect(),
                f,
                info_rect,
            );
        }
    }

    /// Draws the provided list of strings to the info rect
    fn draw_info<'b>(
        vals: Vec<Cow<'b, str>>,
        f: &mut Frame<CrosstermBackend<&mut Stdout>>,
        rect: Rect,
    ) {
        let mut spans = Vec::with_capacity(vals.len());

        // Color anything in brackets
        for v in vals.iter() {
            let s = match v {
                Cow::Borrowed(s) => s,
                Cow::Owned(s) => s.as_str(),
            };

            if s.starts_with('[') {
                if let Some(end) = s.find(']') {
                    spans.push(Spans::from(
                        vec![
                            Span::styled(&s[..end+1], Style::default().fg(Color::Green)),
                            Span::raw(&s[end+1..]),
                        ]
                    ));
                    continue;
                }
            }
            spans.push(Spans::from(Span::raw(s)));
        }

        let footer = Tabs::new(spans)
            .block(Block::default().borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT).border_type(BorderType::Rounded))
            .style(Style::default().fg(Color::DarkGray))
            .divider("|");
        f.render_widget(footer, rect);
    }

    fn draw_home<'b>(
        &'b self,
        _state: &mut State,
        _f: &mut Frame<CrosstermBackend<&mut Stdout>>,
        _rect: Rect,
        _footer: &mut Option<Rect>,
    ) {
    }

    fn draw_userlist<'b>(
        &'b self,
        state: &mut State,
        f: &mut Frame<CrosstermBackend<&mut Stdout>>,
        rect: Rect,
        info: &mut Option<Rect>,
    ) {
        let header_cells = state.user_list_headers.iter().map(|v| Cell::from(v.as_str()));
        let header_row = Row::new(header_cells)
            .height(1)
            .style(Style::default().fg(Color::DarkGray));

        let mut num_changed = 0; 
        let mut on_editable = false;
        let mut editing_info: &[&str] = &[];
        let mut table_spans = Vec::with_capacity(state.user_list.len());
        for y in 0..state.user_list.len() {
            let mut items = Vec::with_capacity(state.user_list_headers.len());
            for x in 0..state.user_list_headers.len() {
                let i = &state.user_list[y][x] as &dyn EditableWidget;
                let cur_focused = y == state.cur_focus.0 && x == state.cur_focus.1;
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
                // Style the current focused item
                if cur_focused {
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
                items.push(Spans::from(spans));
            }
            let row = Row::new(items).height(1);
            if y == state.cur_focus.0 {
                table_spans.push(row.style(Style::default().bg(Color::DarkGray)));
            } else {
                table_spans.push(row);
            }
        }

        let user_table = Table::new(table_spans)
            .block(Block::default().borders(Borders::LEFT | Borders::RIGHT))
            .header(header_row)
            .widths(&[
                Constraint::Percentage(33),
                Constraint::Percentage(33),
                Constraint::Percentage(100),
            ]);
        f.render_stateful_widget(user_table, rect, &mut state.user_list_state);

        if let Some(info_rect) = info.take() {
            let mut info_vals: Vec<Cow<str>> = Vec::new();
            if state.editing {
                info_vals.extend(["[EDITING]"].iter().map(|v| Into::into(*v)));
                info_vals.extend(editing_info.iter().map(|v| Into::into(*v)));
            } else {
                info_vals.extend(DEFAULT_INFO.iter().skip(1).map(|v| Into::into(*v)));
                info_vals.extend(TABLE_INFO.iter().map(|v| Into::into(*v)));
                if on_editable {
                    info_vals.push("[E] Edit".into());
                }
                if num_changed > 0 {
                    info_vals.push(format!("[F2] Apply ({})", num_changed).into());
                }
            }
            
            Self::draw_info(
                info_vals,
                f,
                info_rect,
            );
        }
    }
}
