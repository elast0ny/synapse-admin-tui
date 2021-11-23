use backend::Synapse;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use state::State;
use std::io::Stdout;
use tui::{backend::CrosstermBackend, Terminal};

pub mod common;
use common::*;

pub mod backend;
pub mod state;
pub mod views;
use views::*;

use clap::Parser;

#[derive(Parser)]
#[clap(
    version = "env!(\"CARGO_PKG_VERSION\")",
    author = "env!(\"CARGO_PKG_AUTHOR\")"
)]
/// A terminal admin panel for synapse
struct Args {
    /// The url that points to the synapse server (Default: http://127.0.0.1:8008)
    #[clap(default_value = "http://127.0.0.1:8008")]
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

    let views: &mut [(bool, &mut dyn ViewImpl<State>)] = &mut [
        (false, &mut HomeView::default()),
        (false, &mut UsersView::default()),
    ];
    let backend = Synapse::new(args.host, args.allow_invalid_certs);
    let state = &mut State::from_views(
        views.iter_mut().map(|v| v.1 as &mut dyn ViewImpl<State>),
        backend,
    );

    let mut view_changed = true;
    loop {
        let (entered_once, cur_view): &mut (bool, &mut dyn ViewImpl<State>) =
            &mut views[state.cur_tab()];

        // Call the draw impl
        if view_changed {
            terminal.draw(|f| {
                // If main layout has an empty content
                if let Some(content_rect) = state.draw_base(f, f.size()) {
                    if !*entered_once {
                        cur_view.enter_view(state);
                        *entered_once = true;
                    }
                    // Forward the draw call to the current view
                    cur_view.draw_view(f, content_rect, state);
                }
            })?;
        }

        // Wait for something to happen
        let evt = event::read()?;
        if let Event::Resize(..) = evt {
            view_changed = true;
            continue;
        }

        // Handle any core key else
        let r = state.handle_event_pre(&evt);
        view_changed = matches!(r, HandleRes::ReDraw);
        if view_changed || matches!(r, HandleRes::Handled) {
            continue;
        } else if matches!(r, HandleRes::Exit(_)) {
            break;
        }

        // Forward anything else to the view
        let r = cur_view.handle_event(&evt, state);
        view_changed = matches!(r, HandleRes::ReDraw);
        if view_changed || matches!(r, HandleRes::Handled) {
            continue;
        } else if matches!(r, HandleRes::Exit(_)) {
            break;
        }

        if matches!(state.handle_event_last(&evt), HandleRes::Exit(_)) {
            break;
        }
    }

    Ok(())
}
