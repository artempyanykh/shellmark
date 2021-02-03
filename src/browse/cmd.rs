use std::{
    io::{self, Stderr},
    ops::Range,
};

use anyhow::Result;
use crossterm::{
    event::{Event, KeyEvent, KeyModifiers},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};
use crossterm::{
    event::{EventStream, KeyCode},
    execute,
};
use futures::{stream, TryStreamExt};
use fuzzy_matcher::skim::SkimMatcherV2;
use tokio::{fs, sync::mpsc, time::Instant};
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use tui::{backend::CrosstermBackend, Terminal};

use super::*;
use crate::{
    bookmarks::{read_bookmarks, write_bookmarks},
    cli::OutType,
    search::find_matches,
    storage::simplify_path,
};

pub async fn browse_cmd(out_type: OutType) -> Result<()> {
    setup_terminal()?;
    let output = interact().await?;
    restore_terminal()?;
    if let Some(output) = output {
        print!("{}", prepare_output(&output, out_type));
    }
    Ok(())
}

fn prepare_output(output: &str, out_type: OutType) -> String {
    match out_type {
        OutType::Plain => output.to_string(),
        OutType::Posix => format!("cd '{}'", output),
        OutType::PowerShell => format!("Push-Location '{}'", output),
    }
}

async fn interact() -> Result<Option<String>> {
    let bookmarks = read_bookmarks().await?;
    let matcher = SkimMatcherV2::default();

    let backend = CrosstermBackend::new(io::stderr());
    let mut terminal = Terminal::new(backend)?;

    // Setup an event loop
    let ticks = stream::repeat(Tick)
        .map(SystemEvent::from)
        .throttle(REFRESH_RATE_MS)
        .map(Result::Ok);
    tokio::pin!(ticks);

    let (tx, rx) = mpsc::channel(1);
    let one_off = ReceiverStream::new(rx);

    let user_events = EventStream::new().map_ok(SystemEvent::from);
    let mut system_events = ticks.merge(user_events).merge(one_off);

    let mut app_state = AppState::new(bookmarks);

    loop {
        let event: SystemEvent = TryStreamExt::try_next(&mut system_events)
            .await?
            .expect("Ticks are always present");

        app_state = event_loop(event, app_state, &mut terminal, &matcher).await?;

        if let Some(cmd) = app_state.pending_command.clone() {
            let (new_app_state, output) = handle_command(app_state).await?;
            app_state = new_app_state;
            app_state.pending_command = None;
            if cmd.is_terminal() {
                return Ok(output);
            } else {
                app_state.last_refresh_at = None;
                tx.send(Ok(SystemEvent::from(Tick))).await?;
            }
        }
    }
}

fn setup_terminal() -> Result<()> {
    crossterm::terminal::enable_raw_mode()?;
    Ok(execute!(io::stderr(), EnterAlternateScreen)?)
}

fn restore_terminal() -> Result<()> {
    Ok(execute!(io::stderr(), LeaveAlternateScreen)?)
}

async fn event_loop(
    event: SystemEvent,
    app_state: AppState,
    terminal: &mut Terminal<CrosstermBackend<Stderr>>,
    matcher: &SkimMatcherV2,
) -> Result<AppState> {
    let (should_repaint, new_state) = match event {
        SystemEvent::Timer(_) => match app_state.last_refresh_at {
            None => (
                true,
                AppState {
                    last_refresh_at: Instant::now().into(),
                    ..app_state.clone()
                },
            ),
            Some(_) => (false, app_state.clone()),
        },
        SystemEvent::User(Event::Key(k)) => {
            let mut new_state = handle_key_event(&app_state, k, matcher);
            if new_state != app_state {
                new_state.last_refresh_at = Instant::now().into();
                (true, new_state)
            } else {
                (false, new_state)
            }
        }
        _ => (
            true,
            AppState {
                last_refresh_at: Instant::now().into(),
                ..app_state.clone()
            },
        ),
    };

    if should_repaint {
        ui::draw_ui(terminal, &new_state)?;
    }

    Ok(new_state)
}

fn handle_key_event(app_state: &AppState, event: KeyEvent, matcher: &SkimMatcherV2) -> AppState {
    let mut new_state = match event {
        // Ctrl-C
        KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
        } => AppState {
            pending_command: Some(Command::Exit),
            ..app_state.clone()
        },
        // Ctrl-n, Down
        KeyEvent {
            code: KeyCode::Char('n'),
            modifiers: KeyModifiers::CONTROL,
        }
        | KeyEvent {
            code: KeyCode::Down,
            ..
        } => AppState {
            selection_state: app_state
                .selection_state
                .move_highlight(MoveDirection::Down),
            ..app_state.clone()
        },
        // Ctrl-p, Up
        KeyEvent {
            code: KeyCode::Char('p'),
            modifiers: KeyModifiers::CONTROL,
        }
        | KeyEvent {
            code: KeyCode::Up, ..
        } => AppState {
            selection_state: app_state.selection_state.move_highlight(MoveDirection::Up),
            ..app_state.clone()
        },
        // Enter
        KeyEvent {
            code: KeyCode::Enter,
            modifiers: _,
        } => {
            if let Some(bm) = app_state.selected_bookmark() {
                AppState {
                    pending_command: Some(Command::Enter(bm.dest.clone())),
                    ..app_state.clone()
                }
            } else {
                app_state.clone()
            }
        }
        // Ctrl-k to delete selected bookmark
        KeyEvent {
            code: KeyCode::Char('k'),
            modifiers: m,
        } if m == KeyModifiers::CONTROL | KeyModifiers::SHIFT => {
            if let Some(bm) = app_state.selected_bookmark() {
                AppState {
                    pending_command: Some(Command::DeleteBookmark(bm.clone())),
                    ..app_state.clone()
                }
            } else {
                app_state.clone()
            }
        }
        // Regular chars
        KeyEvent {
            code: KeyCode::Char(c),
            ..
        } => AppState {
            input_state: app_state.input_state.insert_char(c),
            ..app_state.clone()
        },
        // Backspace, Ctrl-Backspace
        KeyEvent {
            code: KeyCode::Backspace,
            modifiers,
        } => {
            if modifiers == KeyModifiers::CONTROL {
                AppState {
                    input_state: InputState::default(),
                    ..app_state.clone()
                }
            } else {
                AppState {
                    input_state: app_state.input_state.delete_char_backwards(),
                    ..app_state.clone()
                }
            }
        }
        _ => app_state.clone(),
    };

    if new_state.input_state != app_state.input_state {
        if new_state.input_state.input.is_empty() {
            new_state.selection_state.selection = Range {
                start: 0,
                end: new_state.bookmarks.len(),
            }
            .collect();
        } else {
            let matches = find_matches(
                matcher,
                &new_state.bookmarks,
                new_state.input_state.to_string(),
            );
            new_state.selection_state.selection = matches;
        }

        if new_state.selection_state.selection.is_empty() {
            new_state.selection_state.highlight = None;
        } else {
            new_state.selection_state.highlight = Some(0);
        }
    }

    new_state
}

async fn handle_command(mut state: AppState) -> Result<(AppState, Option<String>)> {
    match &state.pending_command {
        None => Ok((state, None)),
        Some(cmd) => match cmd.clone() {
            Command::Exit => Ok((state, None)),
            Command::Enter(path) => {
                let path_meta = fs::metadata(&path).await?;
                let dest = if path_meta.is_file() {
                    path.parent()
                        .expect("File should have a parent dir")
                        .to_path_buf()
                } else {
                    path
                };
                Ok((
                    state,
                    Some(simplify_path(&dest).to_string_lossy().to_string()),
                ))
            }
            Command::DeleteBookmark(bm) => {
                state.remove_bookmark(bm.as_ref());
                write_bookmarks(&state.bookmarks).await?;
                Ok((state, None))
            }
            Command::ConfirmDeleteBookmark(_) => {
                unimplemented!("Deletion confirmation not implemented yet")
            }
        },
    }
}
