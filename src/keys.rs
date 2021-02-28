use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub trait Action<C> {
    fn process(&self, key: KeyEvent) -> Option<C>;
}

struct Binding<C, K> {
    check: Box<dyn Fn(KeyEvent) -> Option<K>>,
    act: Box<dyn Fn(K) -> C>,
}

impl<C, K> Binding<C, K> {
    pub fn new<FC, FA>(check: FC, act: FA) -> Binding<C, K>
    where
        FC: Fn(KeyEvent) -> Option<K> + 'static,
        FA: Fn(K) -> C + 'static,
    {
        Binding {
            check: Box::new(check),
            act: Box::new(act),
        }
    }
}

impl<C, K> Action<C> for Binding<C, K> {
    fn process(&self, key: KeyEvent) -> Option<C> {
        (self.check)(key).map(|k| (self.act)(k))
    }
}

pub struct ModeMap<S> {
    map: HashMap<&'static str, Vec<Box<dyn Action<S>>>>,
}

impl<C: Clone + 'static> ModeMap<C> {
    pub fn new() -> ModeMap<C> {
        ModeMap {
            map: HashMap::new(),
        }
    }

    pub fn bind_with_input<M, FC, FA, K>(&mut self, mode: M, check: FC, act: FA)
    where
        M: Into<&'static str>,
        FC: Fn(KeyEvent) -> Option<K> + 'static,
        FA: Fn(K) -> C + 'static,
        K: 'static,
    {
        let binding = Binding::new(check, act);
        self.map
            .entry(mode.into())
            .or_insert(Vec::new())
            .push(Box::new(binding));
    }

    pub fn bind<M, FC>(&mut self, mode: M, check: FC, cmd: C)
    where
        M: Into<&'static str>,
        FC: Fn(KeyEvent) -> bool + 'static,
    {
        let check = move |k| if check(k) { Some(()) } else { None };
        let act = move |_| cmd.clone();
        let binding = Binding {
            check: Box::new(check),
            act: Box::new(act),
        };

        self.map
            .entry(mode.into())
            .or_insert(Vec::new())
            .push(Box::new(binding));
    }

    pub fn process<M: Into<&'static str>>(&self, mode: M, key: KeyEvent) -> Option<C> {
        if let Some(mappings) = self.map.get(mode.into()) {
            for action in mappings {
                if let Some(new_state) = action.process(key) {
                    return Some(new_state);
                }
            }
        }

        None
    }
}

// Common keybindings
// TODO: maybe simplify this with macros

pub fn any_char(key: KeyEvent) -> Option<char> {
    match key {
        KeyEvent {
            code: KeyCode::Char(c),
            modifiers: KeyModifiers::NONE,
        } => Some(c),
        _ => None,
    }
}

pub fn char(ch: char) -> impl Fn(KeyEvent) -> bool {
    move |key| {
        matches!(
            key,
            KeyEvent {
                code: KeyCode::Char(c),
                modifiers: KeyModifiers::NONE
            } if c == ch
        )
    }
}

pub fn ctrl_c(key: KeyEvent) -> bool {
    matches!(
        key,
        KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL
        }
    )
}

pub fn ctrl_n(key: KeyEvent) -> bool {
    matches!(
        key,
        KeyEvent {
            code: KeyCode::Char('n'),
            modifiers: KeyModifiers::CONTROL
        }
    )
}

pub fn ctrl_p(key: KeyEvent) -> bool {
    matches!(
        key,
        KeyEvent {
            code: KeyCode::Char('p'),
            modifiers: KeyModifiers::CONTROL
        }
    )
}

pub fn ctrl_k(key: KeyEvent) -> bool {
    matches!(
        key,
        KeyEvent {
            code: KeyCode::Char('k'),
            modifiers: KeyModifiers::CONTROL
        }
    )
}

#[allow(non_snake_case)]
pub fn ctrl_K(key: KeyEvent) -> bool {
    matches!(
        key,
        KeyEvent {
            code: KeyCode::Char('K'),
            modifiers: KeyModifiers::CONTROL
        }
    )
}

pub fn arrow_down(key: KeyEvent) -> bool {
    matches!(
        key,
        KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::NONE
        }
    )
}

pub fn arrow_up(key: KeyEvent) -> bool {
    matches!(
        key,
        KeyEvent {
            code: KeyCode::Up,
            modifiers: KeyModifiers::NONE
        }
    )
}

pub fn enter(key: KeyEvent) -> bool {
    matches!(
        key,
        KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::NONE
        }
    )
}

pub fn backspace(key: KeyEvent) -> bool {
    matches!(
        key,
        KeyEvent {
            code: KeyCode::Backspace,
            modifiers: KeyModifiers::NONE
        }
    )
}

pub fn ctrl_backspace(key: KeyEvent) -> bool {
    matches!(
        key,
        KeyEvent {
            code: KeyCode::Backspace,
            modifiers: KeyModifiers::CONTROL
        }
    )
}
