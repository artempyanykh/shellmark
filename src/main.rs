use crossterm::{
    event::{Event, EventStream, KeyCode, KeyEvent, KeyModifiers},
    execute, terminal,
};
use futures::stream::{self, TryStreamExt};
use simsearch::SimSearch;
use std::{
    error::Error,
    io::{self, Stdout},
    iter::FromIterator,
    ops::Range,
    path::PathBuf,
    process::exit,
    str::FromStr,
    sync::Arc,
    time::Duration,
    unreachable, usize,
};
use terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use tokio::time::Instant;
use tokio_stream::StreamExt;
use tui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct Bookmark {
    name: String,
    dest: PathBuf,
}

impl Bookmark {
    pub fn new(name: String, dest: PathBuf) -> Bookmark {
        Bookmark { name, dest }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct InputState {
    input: Vec<char>,
    cursor: u16,
}

impl InputState {
    fn insert_char(&self, c: char) -> Self {
        let mut new_state = self.clone();
        new_state.input.insert(new_state.cursor as usize, c);
        new_state.cursor += 1;
        new_state
    }

    fn delete_char_backwards(&self) -> Self {
        let mut new_state = self.clone();
        if new_state.input.is_empty() {
            return new_state;
        }

        new_state.input.remove((new_state.cursor - 1) as usize);
        new_state.cursor -= 1;

        new_state
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SelectionState {
    selection: Vec<usize>,
    highlight: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MoveDirection {
    Down,
    Up,
}

impl MoveDirection {
    fn increment(&self) -> i8 {
        match self {
            MoveDirection::Down => 1,
            MoveDirection::Up => -1,
        }
    }
}

impl SelectionState {
    fn new(selection: Vec<usize>, highlight: Option<usize>) -> Self {
        Self {
            selection,
            highlight,
        }
    }

    fn move_highlight(&self, direction: MoveDirection) -> Self {
        if self.selection.is_empty() {
            return self.clone();
        }
        match self.highlight {
            None => SelectionState {
                highlight: Some(0),
                ..self.clone()
            },
            Some(line) => {
                let increment = direction.increment();
                let new_line = (line as isize + increment as isize)
                    .max(0)
                    .min(self.selection.len() as isize - 1) as usize;

                SelectionState {
                    highlight: Some(new_line),
                    ..self.clone()
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CursorLoc {
    x: u16,
    y: u16,
}

impl CursorLoc {
    fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AppState {
    input_state: InputState,
    selection_state: SelectionState,
    bookmarks: Vec<Arc<Bookmark>>,
    last_refresh_at: Option<Instant>,
}

impl AppState {
    pub fn new(bookmarks: Vec<Arc<Bookmark>>) -> AppState {
        let input_state = InputState::default();
        let selection_state = SelectionState::new(
            Range {
                start: 0,
                end: bookmarks.len() - 1,
            }
            .collect(),
            if bookmarks.is_empty() { None } else { Some(0) },
        );
        AppState {
            input_state,
            selection_state,
            bookmarks,
            last_refresh_at: None,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let dummy_bookmarks = vec![
        Arc::new(Bookmark::new("A".into(), PathBuf::from_str("dest/a")?)),
        Arc::new(Bookmark::new("AB".into(), PathBuf::from_str("dest/ab")?)),
        Arc::new(Bookmark::new("AC".into(), PathBuf::from_str("dest/ac")?)),
        Arc::new(Bookmark::new("B".into(), PathBuf::from_str("dest/b")?)),
        Arc::new(Bookmark::new("C".into(), PathBuf::from_str("dest/c")?)),
        Arc::new(Bookmark::new("D".into(), PathBuf::from_str("dest/d")?)),
        Arc::new(Bookmark::new("E".into(), PathBuf::from_str("dest/e")?)),
        Arc::new(Bookmark::new("F".into(), PathBuf::from_str("dest/f")?)),
        Arc::new(Bookmark::new("G".into(), PathBuf::from_str("dest/g")?)),
        Arc::new(Bookmark::new("H".into(), PathBuf::from_str("dest/h")?)),
    ];

    let mut search = SimSearch::new();
    for (idx, bm) in dummy_bookmarks.iter().enumerate() {
        search.insert(idx, &format!("{} {}", bm.name, bm.dest.to_string_lossy()));
    }

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut app_state = AppState::new(dummy_bookmarks);

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

        app_state = main_loop(event, app_state, &mut terminal, &search).await?;
    }
}

async fn main_loop(
    event: SystemEvent,
    app_state: AppState,
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    search: &SimSearch<usize>,
) -> Result<AppState, Box<dyn Error>> {
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
            let mut new_state = handle_key_event(&app_state, k, search);
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
        draw_ui(terminal, &new_state)?;
    }

    Ok(new_state)
}

fn handle_key_event(app_state: &AppState, event: KeyEvent, search: &SimSearch<usize>) -> AppState {
    let mut new_state = match event {
        KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
        } => {
            exit_app();
            unreachable!();
        }
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
        KeyEvent {
            code: KeyCode::Char(c),
            ..
        } => AppState {
            input_state: app_state.input_state.insert_char(c),
            ..app_state.clone()
        },
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
            new_state.selection_state.selection =
                search.search(&String::from_iter(&new_state.input_state.input));
        }

        if new_state.selection_state.selection.is_empty() {
            new_state.selection_state.highlight = None;
        } else {
            new_state.selection_state.highlight = Some(0);
        }
    }

    new_state
}

fn draw_ui(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    new_state: &AppState,
) -> Result<(), Box<dyn Error>> {
    let mut cursor_loc = CursorLoc::new(0, 0);

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
            Paragraph::new(Span::raw(String::from_iter(&new_state.input_state.input)))
                .alignment(Alignment::Left),
            input_block_area,
        );

        let list_area = Layout::default()
            .horizontal_margin(1)
            .constraints([Constraint::Percentage(100)])
            .split(chunks[1])[0];
        let mut rows = Vec::with_capacity(new_state.selection_state.selection.len());
        for &sel_idx in &new_state.selection_state.selection {
            assert!(
                sel_idx < new_state.bookmarks.len(),
                "Selection index is out of range: {} ∉ ({}, {})",
                sel_idx,
                0,
                new_state.bookmarks.len()
            );
            let bm_name = Cell::from(new_state.bookmarks[sel_idx].name.as_ref())
                .style(Style::default().fg(Color::Green));
            let bm_dest = Cell::from(
                new_state.bookmarks[sel_idx]
                    .dest
                    .to_string_lossy()
                    .to_string(),
            );
            let row = Row::new(vec![bm_name, bm_dest]);
            rows.push(row);
        }
        let bookmarks_tbl = Table::new(rows)
            .block(Block::default())
            .column_spacing(1)
            .widths(&[Constraint::Min(10), Constraint::Min(10)])
            .highlight_symbol(">> ")
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));
        let mut bookmarks_state = TableState::default();
        bookmarks_state.select(new_state.selection_state.highlight);

        f.render_stateful_widget(bookmarks_tbl, list_area, &mut bookmarks_state);

        cursor_loc = CursorLoc::new(
            input_block_area.x + new_state.input_state.cursor,
            input_block_area.y,
        );
    })?;

    terminal.set_cursor(cursor_loc.x, cursor_loc.y)?;
    terminal.show_cursor()?;

    Ok(())
}

#[allow(unused_must_use)] // this is exit anyway
fn exit_app() {
    execute!(io::stdout(), LeaveAlternateScreen);
    exit(0)
}
