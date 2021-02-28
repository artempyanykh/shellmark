use std::io::{self, Stderr};

use anyhow::Result;
use crossterm::{
    event::Event,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};
use crossterm::{event::EventStream, execute};
use futures::{stream, TryStreamExt};
use fuzzy_matcher::skim::SkimMatcherV2;
use tokio::time::Instant;
use tokio_stream::StreamExt;
use tui::{backend::CrosstermBackend, Terminal};

use super::*;
use crate::bookmarks::read_bookmarks;
use crate::keys;
use crate::keys::ModeMap;

pub async fn browse_cmd() -> Result<Option<Action>> {
    setup_terminal()?;
    let output = interact().await;
    restore_terminal()?;
    output
}

async fn interact() -> Result<Option<Action>> {
    let bookmarks = read_bookmarks().await?;
    let matcher = SkimMatcherV2::default();
    let keybinds = setup_keybindings();

    let backend = CrosstermBackend::new(io::stderr());
    let mut terminal = Terminal::new(backend)?;

    // Setup an event loop
    let ticks = stream::repeat(Tick)
        .map(SystemEvent::from)
        .throttle(REFRESH_RATE_MS)
        .map(Result::Ok);
    tokio::pin!(ticks);

    let user_events = EventStream::new().map_ok(SystemEvent::from);
    let mut system_events = ticks.merge(user_events);

    let mut app_state = BrowseState::new(bookmarks, Arc::new(matcher));

    loop {
        let event: SystemEvent = TryStreamExt::try_next(&mut system_events)
            .await?
            .expect("Ticks are always present");

        match event_loop(event, app_state, &keybinds, &mut terminal).await? {
            HandleResult::Continue(new_state) => app_state = new_state,
            HandleResult::Terminate(action) => return Ok(action),
        }
    }
}

fn setup_terminal() -> Result<()> {
    crossterm::terminal::enable_raw_mode()?;
    Ok(execute!(io::stderr(), EnterAlternateScreen)?)
}

fn restore_terminal() -> Result<()> {
    crossterm::terminal::disable_raw_mode()?;
    Ok(execute!(io::stderr(), LeaveAlternateScreen)?)
}

async fn event_loop(
    event: SystemEvent,
    app_state: BrowseState,
    keybinds: &ModeMap<Command>,
    terminal: &mut Terminal<CrosstermBackend<Stderr>>,
) -> Result<HandleResult> {
    let (should_repaint, new_state) = match event {
        SystemEvent::Timer(_) => match app_state.last_refresh_at {
            None => (
                true,
                BrowseState {
                    last_refresh_at: Instant::now().into(),
                    ..app_state.clone()
                },
            ),
            Some(_) => (false, app_state.clone()),
        },
        SystemEvent::User(Event::Key(k)) => {
            let command = keybinds.process(app_state.mode, k);
            let result = match command {
                None => HandleResult::Continue(app_state.clone()),
                Some(command) => app_state.handle_command(&command).await?,
            };
            match result {
                HandleResult::Continue(mut new_state) => {
                    if new_state != app_state {
                        new_state.last_refresh_at = Instant::now().into();
                        (true, new_state)
                    } else {
                        (false, new_state)
                    }
                }
                act @ HandleResult::Terminate(_) => return Ok(act),
            }
        }
        _ => (
            true,
            BrowseState {
                last_refresh_at: Instant::now().into(),
                ..app_state.clone()
            },
        ),
    };

    if should_repaint {
        ui::draw_ui(terminal, &new_state)?;
    }

    Ok(HandleResult::Continue(new_state))
}

fn setup_keybindings() -> ModeMap<Command> {
    let mut mapping = ModeMap::new();

    // Normal mode mappings
    mapping.bind(Mode::Normal, keys::ctrl_c, Command::ExitApp);

    mapping.bind(
        Mode::Normal,
        |k| keys::ctrl_n(k) || keys::arrow_down(k),
        Command::MoveSel(MoveDirection::Down),
    );

    mapping.bind(
        Mode::Normal,
        |k| keys::ctrl_p(k) || keys::arrow_up(k),
        Command::MoveSel(MoveDirection::Up),
    );

    mapping.bind(Mode::Normal, keys::enter, Command::EnterSelDir);

    mapping.bind(
        Mode::Normal,
        |k| keys::ctrl_k(k) || keys::ctrl_K(k),
        Command::EnterMode(Mode::PendingDelete),
    );

    mapping.bind(Mode::Normal, keys::backspace, Command::DeleteCharBack);

    mapping.bind(Mode::Normal, keys::ctrl_backspace, Command::ClearInput);

    mapping.bind_with_input(Mode::Normal, keys::any_char, |c| Command::InsertChar(c));

    // PendingDelete mode mappings
    mapping.bind(Mode::PendingDelete, keys::ctrl_c, Command::ExitApp);
    mapping.bind(
        Mode::PendingDelete,
        keys::char('y'),
        Command::DelSelBookmark,
    );
    mapping.bind(
        Mode::PendingDelete,
        keys::char('n'),
        Command::EnterMode(Mode::Normal),
    );

    mapping
}
