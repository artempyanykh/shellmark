use std::{iter::FromIterator, ops::Range, path::PathBuf, sync::Arc, time::Duration};

use crossterm::event::Event;

use tokio::time::Instant;

use crate::bookmarks::Bookmark;

mod cmd;
mod ui;

pub use cmd::browse_cmd;

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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InputState {
    pub input: Vec<char>,
    pub cursor: u16,
}

impl InputState {
    pub fn insert_char(&self, c: char) -> Self {
        let mut new_state = self.clone();
        new_state.input.insert(new_state.cursor as usize, c);
        new_state.cursor += 1;
        new_state
    }

    pub fn delete_char_backwards(&self) -> Self {
        let mut new_state = self.clone();
        if new_state.input.is_empty() {
            return new_state;
        }

        new_state.input.remove((new_state.cursor - 1) as usize);
        new_state.cursor -= 1;

        new_state
    }

    pub fn to_string(&self) -> String {
        String::from_iter(&self.input)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionState {
    // indices into bookmarks of App state
    pub selection: Vec<usize>,
    // idx into selection
    pub highlight: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveDirection {
    Down,
    Up,
}

impl MoveDirection {
    pub fn increment(&self) -> i8 {
        match self {
            MoveDirection::Down => 1,
            MoveDirection::Up => -1,
        }
    }
}

impl SelectionState {
    pub fn from_bookmarks(bookmarks: &[Arc<Bookmark>]) -> Self {
        let selection = Range {
            start: 0,
            end: bookmarks.len(),
        }
        .collect();
        let highlight = if bookmarks.is_empty() { None } else { Some(0) };
        Self {
            selection,
            highlight,
        }
    }

    pub fn move_highlight(&self, direction: MoveDirection) -> Self {
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
pub struct AppState {
    pub input_state: InputState,
    pub selection_state: SelectionState,
    pub bookmarks: Vec<Arc<Bookmark>>,
    pub pending_command: Option<Command>,
    pub last_refresh_at: Option<Instant>,
}

impl AppState {
    pub fn new(bookmarks: Vec<Arc<Bookmark>>) -> AppState {
        let input_state = InputState::default();
        let selection_state = SelectionState::from_bookmarks(&bookmarks);
        AppState {
            input_state,
            selection_state,
            bookmarks,
            pending_command: None,
            last_refresh_at: None,
        }
    }

    pub fn selected_bookmark(&self) -> Option<Arc<Bookmark>> {
        self.selection_state
            .highlight
            .map(|sel_idx| self.selection_state.selection[sel_idx])
            .map(|b_idx| self.bookmarks[b_idx].clone())
    }

    pub fn remove_bookmark(&mut self, bookmark: &Bookmark) {
        self.bookmarks.retain(|b| *b.as_ref() != *bookmark);
        self.selection_state = SelectionState::from_bookmarks(&self.bookmarks)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Exit,
    Enter(PathBuf),
    #[allow(dead_code)]
    ConfirmDeleteBookmark(Arc<Bookmark>),
    DeleteBookmark(Arc<Bookmark>),
}

impl Command {
    fn is_terminal(&self) -> bool {
        use Command::*;
        match self {
            Exit | Enter(_) => true,
            ConfirmDeleteBookmark(_) | DeleteBookmark(_) => false,
        }
    }
}
