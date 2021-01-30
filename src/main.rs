use crossterm::{
    cursor,
    event::{Event, EventStream, KeyCode, KeyEvent, KeyModifiers},
    execute, terminal, ExecutableCommand,
};
use cursor::{MoveDown, MoveRight};
use futures::stream::{self, TryStreamExt};
use io::{stderr, stdout, Write};
use std::{
    error::Error,
    io::{self, Stdout},
    iter::FromIterator,
    pin::Pin,
    process::exit,
    time::Duration,
};
use tokio_stream::StreamExt;
use tui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::Span,
    widgets::{Block, Borders, Clear, Paragraph},
    Terminal,
};

// Not strictly needed now as there are no background activities not related to terminal events
// But let's keep just in case
const REFRESH_RATE_MS: Duration = Duration::from_millis(1000);

#[derive(Debug, Clone, Copy)]
struct Tick;

#[derive(Debug)]
enum SystemEvent {
    Timer(Tick),
    User(Event),
}

impl From<Event> for SystemEvent {
    fn from(v: Event) -> Self {
        SystemEvent::User(v)
    }
}

impl From<Tick> for SystemEvent {
    fn from(v: Tick) -> Self {
        SystemEvent::Timer(v)
    }
}

struct AppState {
    user_input: Vec<char>,
    cursor_pos: u16,
    // We'll use this later, to play more nicely with pre-existing content
    initial_cursor: (u16, u16),
}

impl AppState {
    fn new(x: u16, y: u16) -> AppState {
        AppState {
            user_input: Vec::new(),
            cursor_pos: 0,
            initial_cursor: (x, y),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    terminal::enable_raw_mode()?;
    let stdout = io::stdout();

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let initial_cursor = terminal.get_cursor()?;
    let mut app_state = AppState::new(initial_cursor.0, initial_cursor.1);

    // For now support only full-screen mode
    terminal.clear()?;

    // Setup an event loop
    let ticks = stream::repeat(Tick)
        .map(SystemEvent::from)
        .throttle(REFRESH_RATE_MS)
        .map(Result::Ok);
    tokio::pin!(ticks);
    let user_events = EventStream::new().map_ok(SystemEvent::from);
    let mut system_events = ticks.merge(user_events);

    loop {
        let event: SystemEvent = TryStreamExt::try_next(&mut system_events)
            .await?
            .expect("Ticks are always present");

        main_loop(event, &mut app_state, &mut terminal).await?;
    }
}

async fn main_loop(
    event: SystemEvent,
    app_state: &mut AppState,
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
) -> Result<(), Box<dyn Error>> {
    match event {
        SystemEvent::Timer(_) => (),
        SystemEvent::User(Event::Key(k)) => handle_key_event(app_state, k, terminal),
        _ => (),
    }

    terminal.draw(|f| {
        let all_area = f.size();
        let block = Block::default().title("Shellmark").borders(Borders::ALL);
        let block_inner = block.inner(all_area);
        f.render_widget(block, all_area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Min(1)])
            .margin(0)
            .split(block_inner);

        let input_area = chunks[0];
        let input_block = Block::default().borders(Borders::BOTTOM);
        let input_block_area = input_block.inner(chunks[0]);

        let input_area_chunk = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(3), Constraint::Min(2)])
            .split(input_block_area);
        let input_symbol_area = input_area_chunk[0];
        let input_block_area = input_area_chunk[1];

        f.render_widget(input_block, input_area);
        f.render_widget(
            Paragraph::new(Span::raw(">")).alignment(Alignment::Center),
            input_symbol_area,
        );

        f.render_widget(
            Paragraph::new(Span::raw(String::from_iter(&app_state.user_input)))
                .alignment(Alignment::Left),
            input_block_area,
        );

        f.set_cursor(
            input_block_area.x + app_state.cursor_pos,
            input_block_area.y,
        );

        let list_area = chunks[1];
    })?;

    Ok(())
}

fn handle_key_event(
    app_state: &mut AppState,
    event: KeyEvent,
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
) {
    match event {
        KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
        } => exit_app(terminal),
        KeyEvent {
            code: KeyCode::Char(c),
            ..
        } => {
            app_state.user_input.insert(app_state.cursor_pos.into(), c);
            app_state.cursor_pos += 1;
        }
        KeyEvent {
            code: KeyCode::Backspace,
            modifiers,
        } => {
            if modifiers == KeyModifiers::CONTROL {
                app_state.user_input.clear();
                app_state.cursor_pos = 0;
            } else {
                if app_state.cursor_pos > 0 {
                    app_state
                        .user_input
                        .remove((app_state.cursor_pos - 1).into());
                    app_state.cursor_pos -= 1;
                }
            }
        }
        _ => (),
    }
}

#[allow(unused_must_use)] // this is exit anyway
fn exit_app(terminal: &mut Terminal<CrosstermBackend<Stdout>>) {
    terminal.clear();
    terminal.set_cursor(0, 0);
    terminal.show_cursor();
    exit(0)
}
