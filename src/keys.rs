use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub trait Action<C> {
    fn process(&self, key: KeyEvent) -> Option<C>;
    fn desc(&self) -> Option<(&str, &str)>;
}

pub struct Combo<K> {
    pub check: Box<dyn Fn(KeyEvent) -> Option<K>>,
    pub desc: Option<String>,
}

impl<K> Combo<K> {
    pub fn with_input<FC>(check: FC, desc: Option<String>) -> Self
    where
        FC: Fn(KeyEvent) -> Option<K> + 'static,
    {
        Combo {
            check: Box::new(check),
            desc,
        }
    }
}

impl Combo<()> {
    pub fn with_match<FC>(check: FC, desc: Option<String>) -> Self
    where
        FC: Fn(KeyEvent) -> bool + 'static,
    {
        Combo {
            check: Box::new(move |k| if check(k) { Some(()) } else { None }),
            desc,
        }
    }
}

struct Binding<C, K> {
    combo: Combo<K>,
    act: Box<dyn Fn(K) -> C>,
    desc: Option<String>,
}

impl<C, K> Binding<C, K> {
    pub fn new<FA>(combo: Combo<K>, act: FA, desc: Option<String>) -> Binding<C, K>
    where
        FA: Fn(K) -> C + 'static,
    {
        Binding {
            combo,
            act: Box::new(act),
            desc,
        }
    }
}

impl<C, K> Action<C> for Binding<C, K> {
    fn process(&self, key: KeyEvent) -> Option<C> {
        (self.combo.check)(key).map(|k| (self.act)(k))
    }

    fn desc(&self) -> Option<(&str, &str)> {
        match (self.combo.desc.as_deref(), &self.desc.as_deref()) {
            (Some(combo_desc), Some(action_desc)) => Some((combo_desc, action_desc)),
            _ => None,
        }
    }
}

pub struct ModeMap<S> {
    pub map: HashMap<&'static str, Vec<Box<dyn Action<S>>>>,
}

impl<C: Clone + 'static> ModeMap<C> {
    pub fn new() -> ModeMap<C> {
        ModeMap {
            map: HashMap::new(),
        }
    }

    pub fn bind_with_input<M, K, FA>(
        &mut self,
        mode: M,
        combo: Combo<K>,
        act: FA,
        desc: Option<String>,
    ) where
        M: Into<&'static str>,
        FA: Fn(K) -> C + 'static,
        K: 'static,
    {
        let binding = Binding::new(combo, act, desc);
        self.map
            .entry(mode.into())
            .or_insert_with(Vec::new)
            .push(Box::new(binding));
    }

    pub fn bind_with_desc<M>(&mut self, mode: M, combo: Combo<()>, cmd: C, desc: Option<String>)
    where
        M: Into<&'static str>,
    {
        let act = move |_| cmd.clone();
        let binding = Binding::new(combo, Box::new(act), desc);

        self.map
            .entry(mode.into())
            .or_insert_with(Vec::new)
            .push(Box::new(binding));
    }

    pub fn bind<M>(&mut self, mode: M, combo: Combo<()>, cmd: C, desc: &str)
    where
        M: Into<&'static str>,
    {
        self.bind_with_desc(mode, combo, cmd, Some(desc.to_string()))
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

pub fn any_char() -> Combo<char> {
    Combo::with_input(
        |key| match key {
            KeyEvent {
                code: KeyCode::Char(c),
                modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            } => Some(c),
            _ => None,
        },
        None,
    )
}

pub fn char(ch: char) -> Combo<()> {
    Combo::with_match(
        move |key| {
            matches!(
                key,
                KeyEvent {
                    code: KeyCode::Char(c),
                    modifiers: KeyModifiers::NONE
                } if c == ch
            )
        },
        Some(format!("{}", ch)),
    )
}

pub fn ctrl_c() -> Combo<()> {
    Combo::with_match(
        |key: KeyEvent| {
            matches!(
                key,
                KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::CONTROL
                }
            )
        },
        Some("C-c".to_string()),
    )
}

pub fn ctrl_n() -> Combo<()> {
    Combo::with_match(
        |key: KeyEvent| {
            matches!(
                key,
                KeyEvent {
                    code: KeyCode::Char('n'),
                    modifiers: KeyModifiers::CONTROL
                }
            )
        },
        Some("C-n".to_string()),
    )
}

pub fn ctrl_p() -> Combo<()> {
    Combo::with_match(
        |key: KeyEvent| {
            matches!(
                key,
                KeyEvent {
                    code: KeyCode::Char('p'),
                    modifiers: KeyModifiers::CONTROL
                }
            )
        },
        Some("C-p".to_string()),
    )
}

pub fn ctrl_k() -> Combo<()> {
    Combo::with_match(
        |key: KeyEvent| {
            matches!(
                key,
                KeyEvent {
                    code: KeyCode::Char('k'),
                    modifiers: KeyModifiers::CONTROL
                }
            )
        },
        Some("C-k".to_string()),
    )
}

#[allow(non_snake_case)]
pub fn ctrl_K() -> Combo<()> {
    Combo::with_match(
        |key: KeyEvent| {
            matches!(
                key,
                KeyEvent {
                    code: KeyCode::Char('K'),
                    modifiers: KeyModifiers::CONTROL
                }
            )
        },
        Some("C-K".to_string()),
    )
}

pub fn arrow_down() -> Combo<()> {
    Combo::with_match(
        |key: KeyEvent| {
            matches!(
                key,
                KeyEvent {
                    code: KeyCode::Down,
                    modifiers: KeyModifiers::NONE
                }
            )
        },
        Some("Down".to_string()),
    )
}

pub fn arrow_up() -> Combo<()> {
    Combo::with_match(
        |key: KeyEvent| {
            matches!(
                key,
                KeyEvent {
                    code: KeyCode::Up,
                    modifiers: KeyModifiers::NONE
                }
            )
        },
        Some("Up".to_string()),
    )
}

pub fn enter() -> Combo<()> {
    Combo::with_match(
        |key: KeyEvent| {
            matches!(
                key,
                KeyEvent {
                    code: KeyCode::Enter,
                    modifiers: KeyModifiers::NONE
                }
            )
        },
        Some("Enter".to_string()),
    )
}

pub fn backspace() -> Combo<()> {
    Combo::with_match(
        |key: KeyEvent| {
            matches!(
                key,
                KeyEvent {
                    code: KeyCode::Backspace,
                    modifiers: KeyModifiers::NONE
                }
            )
        },
        Some("Backspace".to_string()),
    )
}

pub fn ctrl_backspace() -> Combo<()> {
    Combo::with_match(
        |key: KeyEvent| {
            matches!(
                key,
                KeyEvent {
                    code: KeyCode::Backspace,
                    modifiers: KeyModifiers::CONTROL
                }
            )
        },
        Some("C-Backspace".to_string()),
    )
}

pub fn f1() -> Combo<()> {
    Combo::with_match(
        |key: KeyEvent| {
            matches!(
                key,
                KeyEvent {
                    code: KeyCode::F(1),
                    modifiers: KeyModifiers::NONE
                }
            )
        },
        Some("F1".to_string()),
    )
}

pub fn esc() -> Combo<()> {
    Combo::with_match(
        |key: KeyEvent| {
            matches!(
                key,
                KeyEvent {
                    code: KeyCode::Esc,
                    modifiers: KeyModifiers::NONE
                }
            )
        },
        Some("Esc".to_string()),
    )
}
