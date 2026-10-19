#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KeyboardPresentation {
    #[default]
    Viewport,
    WaylandLayer,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyboardKey {
    Character(char),
    Backspace,
    Enter,
    Escape,
    Shift,
    Space,
    Tab,
}

pub const KEY_ROWS: &[&[KeyboardKey]] = &[
    &[
        KeyboardKey::Character('1'),
        KeyboardKey::Character('2'),
        KeyboardKey::Character('3'),
        KeyboardKey::Character('4'),
        KeyboardKey::Character('5'),
        KeyboardKey::Character('6'),
        KeyboardKey::Character('7'),
        KeyboardKey::Character('8'),
        KeyboardKey::Character('9'),
        KeyboardKey::Character('0'),
        KeyboardKey::Character('-'),
        KeyboardKey::Character('='),
        KeyboardKey::Backspace,
    ],
    &[
        KeyboardKey::Character('q'),
        KeyboardKey::Character('w'),
        KeyboardKey::Character('e'),
        KeyboardKey::Character('r'),
        KeyboardKey::Character('t'),
        KeyboardKey::Character('y'),
        KeyboardKey::Character('u'),
        KeyboardKey::Character('i'),
        KeyboardKey::Character('o'),
        KeyboardKey::Character('p'),
        KeyboardKey::Character('['),
        KeyboardKey::Character(']'),
        KeyboardKey::Character('\\'),
    ],
    &[
        KeyboardKey::Character('a'),
        KeyboardKey::Character('s'),
        KeyboardKey::Character('d'),
        KeyboardKey::Character('f'),
        KeyboardKey::Character('g'),
        KeyboardKey::Character('h'),
        KeyboardKey::Character('j'),
        KeyboardKey::Character('k'),
        KeyboardKey::Character('l'),
        KeyboardKey::Character(';'),
        KeyboardKey::Character('\''),
        KeyboardKey::Enter,
    ],
    &[
        KeyboardKey::Shift,
        KeyboardKey::Character('z'),
        KeyboardKey::Character('x'),
        KeyboardKey::Character('c'),
        KeyboardKey::Character('v'),
        KeyboardKey::Character('b'),
        KeyboardKey::Character('n'),
        KeyboardKey::Character('m'),
        KeyboardKey::Character(','),
        KeyboardKey::Character('.'),
        KeyboardKey::Character('/'),
    ],
    &[KeyboardKey::Tab, KeyboardKey::Space, KeyboardKey::Escape],
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyboardSession {
    pub open: bool,
    pub status: Option<String>,
    pub presentation: KeyboardPresentation,
    pub selected_row: usize,
    pub selected_column: usize,
    pub shift: bool,
}

impl Default for KeyboardSession {
    fn default() -> Self {
        Self {
            open: false,
            status: None,
            presentation: KeyboardPresentation::default(),
            selected_row: 2,
            selected_column: 0,
            shift: false,
        }
    }
}

impl KeyboardSession {
    pub fn selected_index(&self) -> usize {
        KEY_ROWS[..self.selected_row]
            .iter()
            .map(|row| row.len())
            .sum::<usize>()
            + self.selected_column
    }

    pub fn selected_key(&self) -> KeyboardKey {
        KEY_ROWS[self.selected_row][self.selected_column]
    }

    pub fn move_selection(&mut self, row_delta: isize, column_delta: isize) {
        if column_delta != 0 {
            let row_len = KEY_ROWS[self.selected_row].len() as isize;
            self.selected_column =
                (self.selected_column as isize + column_delta).rem_euclid(row_len) as usize;
            return;
        }

        self.selected_row =
            (self.selected_row as isize + row_delta).clamp(0, KEY_ROWS.len() as isize - 1) as usize;
        self.selected_column = self
            .selected_column
            .min(KEY_ROWS[self.selected_row].len() - 1);
    }
}

impl KeyboardKey {
    pub fn label(self, shift: bool) -> String {
        match self.character(shift) {
            Some(character) => character.to_string(),
            None => match self {
                Self::Backspace => "⌫".to_owned(),
                Self::Enter => "Enter".to_owned(),
                Self::Escape => "Esc".to_owned(),
                Self::Shift => "⇧".to_owned(),
                Self::Space => "Espaço".to_owned(),
                Self::Tab => "Tab".to_owned(),
                Self::Character(_) => unreachable!(),
            },
        }
    }

    pub fn character(self, shift: bool) -> Option<char> {
        let Self::Character(character) = self else {
            return None;
        };
        Some(if shift {
            match character {
                'a'..='z' => character.to_ascii_uppercase(),
                '1' => '!',
                '2' => '@',
                '3' => '#',
                '4' => '$',
                '5' => '%',
                '6' => '^',
                '7' => '&',
                '8' => '*',
                '9' => '(',
                '0' => ')',
                '-' => '_',
                '=' => '+',
                '[' => '{',
                ']' => '}',
                ';' => ':',
                '\'' => '"',
                '\\' => '|',
                ',' => '<',
                '.' => '>',
                '/' => '?',
                _ => character,
            }
        } else {
            character
        })
    }
}
