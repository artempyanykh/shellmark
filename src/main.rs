use clap::{crate_version, Clap};
use crossterm::{
    event::{Event, EventStream, KeyCode, KeyEvent, KeyModifiers},
    execute, terminal,
};
use futures::stream::{self, TryStreamExt};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
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
    unimplemented, unreachable, usize,
};
use terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use tokio::time::Instant;
use tokio_stream::StreamExt;
use tui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Terminal,
};

// Not strictly needed now as there are no background activities not related to terminal events
// But let's keep just in case
const REFRESH_RATE_MS: Duration = Duration::from_millis(1000);

#[derive(Clap)]
#[clap(version = crate_version!())]
/// Cross-platform CLI bookmarks manager.
struct Opts {
    #[clap(subcommand)]
    command: Option<Command>,
}

#[derive(Clap)]
enum Command {
    /// Add bookmarks
    Add(AddCmd),
    /// (default) Interactively find and select bookmarks
    Browse(BrowseCmd),
}

#[derive(Clap)]
struct AddCmd {
    #[clap(short, long)]
    /// Replace the bookmark's destination when similarly named bookmark exists
    force: bool,
}

#[derive(Clap)]
struct BrowseCmd {}

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
    let opts = Opts::parse();

    match opts.command {
        Some(Command::Add(add_cmd_opts)) => add_cmd(add_cmd_opts).await,
        Some(Command::Browse(_)) | None => browse_cmd().await,
    }
}

async fn add_cmd(add_cmd_opts: AddCmd) -> Result<(), Box<dyn Error>> {
    unimplemented!()
}

async fn browse_cmd() -> Result<(), Box<dyn Error>> {
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
        Arc::new(Bookmark::new(
            "dotfiles".into(),
            PathBuf::from_str("dest/dotfiles")?,
        )),
        Arc::new(Bookmark::new(
            "init.el".into(),
            PathBuf::from_str("dest/dotfiles/emacs.d/init.el")?,
        )),
        Arc::new(Bookmark::new("E".into(), PathBuf::from_str("dest/e")?)),
        Arc::new(Bookmark::new("F".into(), PathBuf::from_str("dest/f")?)),
        Arc::new(Bookmark::new("G".into(), PathBuf::from_str("dest/g")?)),
        Arc::new(Bookmark::new("H".into(), PathBuf::from_str("dest/h")?)),
    ];

    let matcher = SkimMatcherV2::default();

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

        app_state = main_loop(event, app_state, &mut terminal, &matcher).await?;
    }
}

async fn main_loop(
    event: SystemEvent,
    app_state: AppState,
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    matcher: &SkimMatcherV2,
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
        draw_ui(terminal, &new_state)?;
    }

    Ok(new_state)
}

fn handle_key_event(app_state: &AppState, event: KeyEvent, matcher: &SkimMatcherV2) -> AppState {
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
            let matches = find_matches(matcher, &new_state);
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

fn find_matches(matcher: &SkimMatcherV2, state: &AppState) -> Vec<usize> {
    let pattern = String::from_iter(&state.input_state.input);
    // Rank all bookmarks using fuzzy matcher
    let mut scores: Vec<_> = state
        .bookmarks
        .iter()
        .map(|bm| {
            matcher.fuzzy_match(
                &format!("{} {}", bm.name, bm.dest.to_string_lossy()),
                &pattern,
            )
        })
        .enumerate()
        .collect();
    // Reverse sort the scores
    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    // Pick the matches starting from the "best" one
    let mut matches = Vec::new();
    for (idx, score) in &scores {
        if let &Some(score) = score {
            if score > 0 {
                matches.push(*idx);
            }
        }
    }

    matches
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
            // Render bookmark name with some colorization
            let bm_name = colorize_match(
                &new_state.bookmarks[sel_idx].name,
                &new_state.input_state.input,
            );
            let bm_name = Cell::from(bm_name).style(Style::default().fg(Color::Green));
            // Render bookmark dest with some colorization
            let bm_dest = colorize_match(
                &new_state.bookmarks[sel_idx].dest.to_string_lossy(),
                &new_state.input_state.input,
            );
            let bm_dest = Cell::from(bm_dest);
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

fn colorize_match(str: &str, input: &[char]) -> Spans<'static> {
    let mut spans = Vec::new();
    let mut cur_span: Option<(bool, Vec<char>)> = None;
    let mut match_idx = 0;

    for ch in str.chars() {
        if match_idx < input.len()
            && ch.to_lowercase().to_string() == input[match_idx].to_lowercase().to_string()
        {
            // We have a match
            match &mut cur_span {
                None => cur_span = Some((true, vec![ch])),
                Some(existing_span) => {
                    if existing_span.0 {
                        existing_span.1.push(ch);
                    } else {
                        spans.push(colorize_span(existing_span));
                        cur_span = Some((true, vec![ch]));
                    }
                }
            }

            match_idx += 1;
        } else {
            // No match
            match &mut cur_span {
                None => cur_span = Some((false, vec![ch])),
                Some(existing_span) => {
                    if !existing_span.0 {
                        existing_span.1.push(ch);
                    } else {
                        spans.push(colorize_span(existing_span));
                        cur_span = Some((false, vec![ch]));
                    }
                }
            }
        }
    }

    if let Some(span) = cur_span {
        spans.push(colorize_span(&span));
    }

    Spans::from(spans)
}

fn colorize_span(span: &(bool, Vec<char>)) -> Span<'static> {
    let (is_match, text) = span;
    let str = String::from_iter(text);
    if *is_match {
        Span::styled(str, Style::default().fg(Color::Red))
    } else {
        Span::raw(str)
    }
}

#[allow(unused_must_use)] // this is exit anyway
fn exit_app() {
    execute!(io::stdout(), LeaveAlternateScreen);
    exit(0)
}
