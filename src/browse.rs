use std::{
    convert::From, iter::FromIterator, ops::Range, path::PathBuf, sync::Arc, time::Duration,
    unimplemented,
};

use anyhow::Result;

use derivative::Derivative;

use crossterm::event::Event;

use fuzzy_matcher::skim::SkimMatcherV2;
use tokio::time::Instant;

use crate::{
    bookmarks::{write_bookmarks, Bookmark},
    search, shell,
    storage::simplify_path,
};

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
pub struct Input {
    pub input: Vec<char>,
    pub cursor: u16,
}

impl Input {
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
pub struct Selection {
    // indices into bookmarks of App state
    pub candidates: Vec<usize>,
    // idx into selection
    pub selected: Option<usize>,
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

impl Selection {
    pub fn from_bookmarks_with_selected(
        bookmarks: &[Arc<Bookmark>],
        selected: Option<usize>,
    ) -> Self {
        let candidates = Range {
            start: 0,
            end: bookmarks.len(),
        }
        .collect();
        Self::from_candidates_with_selected(candidates, selected)
    }

    pub fn from_bookmarks(bookmarks: &[Arc<Bookmark>]) -> Self {
        Self::from_bookmarks_with_selected(bookmarks, None)
    }

    pub fn from_candidates_with_selected(candidates: Vec<usize>, selected: Option<usize>) -> Self {
        let selected = if candidates.is_empty() {
            None
        } else {
            selected
                .map(|cur| cur.min(candidates.len() - 1))
                .or(Some(0))
        };
        Self {
            candidates,
            selected,
        }
    }

    pub fn move_highlight(&self, direction: &MoveDirection) -> Self {
        if self.candidates.is_empty() {
            return self.clone();
        }
        match self.selected {
            None => Selection {
                selected: Some(0),
                ..self.clone()
            },
            Some(line) => {
                let increment = direction.increment();
                let new_line = (line as isize + increment as isize)
                    .max(0)
                    .min(self.candidates.len() as isize - 1)
                    as usize;

                Selection {
                    selected: Some(new_line),
                    ..self.clone()
                }
            }
        }
    }
}

#[derive(Derivative)]
#[derivative(Debug, Clone, PartialEq, Eq)]
pub struct BrowseState {
    pub bookmarks: Vec<Arc<Bookmark>>,
    #[derivative(Debug = "ignore", PartialEq = "ignore")]
    pub matcher: Arc<SkimMatcherV2>,
    pub input: Input,
    pub selection: Selection,
    pub last_refresh_at: Option<Instant>,
}

pub enum HandleResult {
    Continue(BrowseState),
    Terminate(Option<Action>),
}

impl BrowseState {
    pub fn new(bookmarks: Vec<Arc<Bookmark>>, matcher: Arc<SkimMatcherV2>) -> BrowseState {
        let input = Input::default();
        let selection = Selection::from_bookmarks(&bookmarks);
        BrowseState {
            bookmarks,
            matcher,
            input,
            selection,
            last_refresh_at: None,
        }
    }

    pub async fn handle_command(&self, cmd: &Command) -> Result<HandleResult> {
        match cmd {
            Command::ExitApp => Ok(HandleResult::Terminate(None)),
            Command::EnterSelDir => {
                if let Some(bm) = self.selected_bookmark() {
                    Ok(HandleResult::Terminate(Some(Action::ChangeDirAction {
                        dest: bm.dest.clone(),
                    })))
                } else {
                    Ok(HandleResult::Continue(self.clone()))
                }
            }
            Command::ConfirmDelSelBookmark => {
                unimplemented!()
            }
            Command::DelSelBookmark => {
                let mut new_state = self.clone();
                if let Some(bm) = new_state.selected_bookmark() {
                    new_state.remove_bookmark(&bm);
                    write_bookmarks(&new_state.bookmarks).await?;
                }
                Ok(HandleResult::Continue(new_state))
            }
            Command::InsertChar(c) => {
                let mut new_state = BrowseState {
                    input: self.input.insert_char(*c),
                    ..self.clone()
                };
                new_state.update_selection();
                Ok(HandleResult::Continue(new_state))
            }
            Command::DeleteCharBack => {
                let mut new_state = BrowseState {
                    input: self.input.delete_char_backwards(),
                    ..self.clone()
                };
                new_state.update_selection();
                Ok(HandleResult::Continue(new_state))
            }
            Command::ClearInput => {
                let mut new_state = BrowseState {
                    input: Input::default(),
                    ..self.clone()
                };
                new_state.update_selection();
                Ok(HandleResult::Continue(new_state))
            }
            Command::MoveSel(direction) => {
                let new_selection = self.selection.move_highlight(direction);
                Ok(HandleResult::Continue(BrowseState {
                    selection: new_selection,
                    ..self.clone()
                }))
            }
            Command::ShowHelp => {
                unimplemented!()
            }
        }
    }

    pub fn selected_bookmark(&self) -> Option<Arc<Bookmark>> {
        self.selection
            .selected
            .map(|sel_idx| self.selection.candidates[sel_idx])
            .map(|b_idx| self.bookmarks[b_idx].clone())
    }

    pub fn remove_bookmark(&mut self, bookmark: &Bookmark) {
        self.bookmarks.retain(|b| *b.as_ref() != *bookmark);
        self.update_selection();
    }

    pub fn update_selection(&mut self) {
        let input = self.input.to_string();
        let selection = if input.is_empty() {
            Selection::from_bookmarks_with_selected(&self.bookmarks, self.selection.selected)
        } else {
            let candidates = search::find_matches(&self.matcher, &self.bookmarks, input);
            Selection::from_candidates_with_selected(candidates, self.selection.selected)
        };
        self.selection = selection;
    }
}

#[derive(Clone)]
pub enum Command {
    ExitApp,
    EnterSelDir,
    #[allow(dead_code)]
    ConfirmDelSelBookmark,
    DelSelBookmark,
    InsertChar(char),
    DeleteCharBack,
    ClearInput,
    MoveSel(MoveDirection),
    #[allow(dead_code)]
    ShowHelp,
}

#[allow(dead_code)]
pub enum Mode {
    Normal,
    PendingDelete,
}

pub enum Action {
    ChangeDirAction { dest: PathBuf },
}

impl shell::Output for Action {
    fn to_output(&self, out_type: shell::OutputType) -> Option<String> {
        use shell::OutputType::*;

        match self {
            Action::ChangeDirAction { dest } => {
                let dest_string = simplify_path(dest).to_string_lossy();

                let out = match out_type {
                    Plain => dest_string.to_string(),
                    Posix | Fish => format!("cd {}", dest_string),
                    PowerShell => format!("Push-Location '{}'", dest_string),
                };
                Some(out)
            }
        }
    }
}
