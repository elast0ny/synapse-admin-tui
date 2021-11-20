use std::io::Stdout;

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tui::{backend::CrosstermBackend, Terminal};

mod state;
use state::*;
mod view;
use view::*;

mod editable;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let r = raw_mode_main(&mut stdout);
    execute!(stdout, LeaveAlternateScreen)?;
    disable_raw_mode()?;
    r
}

fn raw_mode_main(stdout: &mut Stdout) -> Result<(), Box<dyn std::error::Error>> {
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let mut state = State::default();
    state.user_list.push(vec![
        "Tony".into(),
        "tony@gmail.com".to_string().into(),
        (&mut true).into(),
    ]);
    state.user_list.push(vec![
        "Liz".into(),
        "Liz@hotmail.ca".to_string().into(),
        (&mut false).into(),
    ]);
    state.user_list.push(vec![
        "Simon".into(),
        "Simon@msn.com".to_string().into(),
        (&mut false).into(),
    ]);

    let mut view = TAB_ORDER[0];

    //let mut key_log = std::fs::OpenOptions::new().create(true).append(true).open("key.log")?;
    loop {
        terminal.draw(|f| view.draw(&mut state, f))?;

        if let Event::Key(key) = event::read()? {
            //key_log.write_fmt(format_args!("{:?} {:?}\n", key.code, key.modifiers))?;

            // If we are currently editing
            if state.editing {
                // We are focused on an editable widget
                match state.cur_focused(&view) {
                    Some(h) if h.is_editable() => {
                        // Forward the key events to the widget
                        if let Some(continue_edit) = h.handle_event(&key) {
                            if !continue_edit {
                                state.editing = false;
                            }
                            continue;
                        }
                    }
                    // Currently editing but no focused editable widget ?
                    _ => state.editing = false,
                }
            }

            match key.code {
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => return Ok(()),
                KeyCode::Char('e') | KeyCode::Char('E') => state.toggle_edit(&view),
                KeyCode::Down => state.inc_row(&view, 1),
                KeyCode::Up => state.prev_row(&view, 1),
                KeyCode::PageDown => state.inc_row(&view, 5),
                KeyCode::PageUp => state.prev_row(&view, 5),
                KeyCode::Left => state.prev_col(&view, 1),
                KeyCode::Right => state.inc_col(&view, 1),
                KeyCode::Tab => state.next_view(&mut view),
                KeyCode::BackTab => state.prev_view(&mut view),
                KeyCode::F(1) => {state.show_help = !state.show_help;},
                _ => {}
            }
        }
    }
}
