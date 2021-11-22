use std::io::Stdout;

use backend::*;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use editable::{Editable, EditableWidget};
use tui::{backend::CrosstermBackend, Terminal};

mod state;
use state::*;
mod view;
use view::*;
mod backend;
mod editable;

use crate::editable::EvtResult;

use clap::Parser;

#[derive(Parser)]
#[clap(
    version = "env!(\"CARGO_PKG_VERSION\")",
    author = "env!(\"CARGO_PKG_AUTHOR\")"
)]
/// A terminal admin panel for synapse
struct Args {
    /// The url that points to the synapse server (Default: http://127.0.0.1:8008)
    #[clap(default_value="http://127.0.0.1:8008")]
    host: String,

    /// Ignore invalid TLS certificates
    #[clap(long)]
    allow_invalid_certs: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let r = raw_mode_main(args, &mut stdout);
    execute!(stdout, LeaveAlternateScreen)?;
    disable_raw_mode()?;
    r
}

fn raw_mode_main(args: Args, stdout: &mut Stdout) -> Result<(), Box<dyn std::error::Error>> {
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let mut state = State::new(Backend::Synapse(Synapse::new(
        args.host,
        args.allow_invalid_certs,
    )));
    let mut view = View::Home;

    //let mut key_log = std::fs::OpenOptions::new().create(true).append(true).open("key.log")?;
    loop {
        state.check_backend_prompt(&mut view);

        terminal.draw(|f| view.draw(&mut state, f))?;

        if let Event::Key(key) = event::read()? {
            //key_log.write_fmt(format_args!("{:?} {:?}\n", key.code, key.modifiers))?;

            let (edit_widget, handler_fn): (
                Option<&mut Editable>,
                fn(&mut State, &mut View, &KeyEvent) -> EvtResult,
            ) = match &view {
                View::Prompt => (state.prompt.cur_field(), handle_event_prompt),
                View::Home => (None, handle_event_passthrough),
                View::UserList => (state.user_state.editing_item(), handle_event_users),
            };

            // Let the widget handle the keystroke
            if let Some(w) = edit_widget {
                if matches!(w.handle_event(&key), EvtResult::Continue | EvtResult::Stop) {
                    continue;
                }
            }

            // See if the current view wants to handle this event
            match handler_fn(&mut state, &mut view, &key) {
                EvtResult::Continue => continue,
                EvtResult::Stop => return Ok(()),
                EvtResult::Pass => {}
            };

            // Fallback to generic impl
            match key.code {
                // Common behavior across all views
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => return Ok(()),
                KeyCode::Tab => state.next_view(&mut view),
                KeyCode::BackTab => state.prev_view(&mut view),
                KeyCode::F(1) => state.toggle_help(),
                _ => {}
            };
        }
    }
}
