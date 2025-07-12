use std::{collections::HashMap, usize};

use ratatui::widgets::{ScrollbarState, TableState};
use tui_textarea::TextArea;

use crate::{app::ITEM_HEIGHT, registry};

pub type InputValidateFn = dyn Fn(&str) -> Result<(), String>;
pub type InputConfirmFn = dyn Fn(String) -> Option<AppMessage>;

pub struct InputState {
    pub label: String,
    pub textarea: TextArea<'static>,

    pub confirm: bool,

    pub validate_fn: Option<Box<InputValidateFn>>,
    pub confirm_fn: Option<Box<InputConfirmFn>>,
}

impl InputState {
    fn new() -> Self {
        Self { label: String::from("No Input Required"), textarea: TextArea::default(), confirm: false, validate_fn: None, confirm_fn: None }
    }

    pub fn text(&self) -> String {
        self.textarea.lines().join("\n")
    }

    pub fn validate(&self) -> Option<Result<(), String>> {
        let text = self.text();

        match self.validate_fn {
            Some(ref validate_fn) => Some((validate_fn)(text.as_str())),
            None => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AppMessageType {
    Info,
    Error,
}

pub struct AppMessage {
    pub ty: AppMessageType,
    pub message: String,
}

impl AppMessage {
    fn new(ty: AppMessageType, message: impl Into<String>) -> Self {
        Self { ty, message: message.into() }
    }

    pub fn info(message: impl Into<String>) -> Self {
        Self::new(AppMessageType::Info, message)
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self::new(AppMessageType::Error, message)
    }
}

pub struct ScrollableTableState {
    pub state: TableState,
    pub scroll: ScrollbarState,

    pub content_length: usize,
}

impl ScrollableTableState {
    fn new(content_length: usize) -> Self {
        Self {
            state: TableState::default().with_selected(0),
            scroll: ScrollbarState::new(content_length),

            content_length,
        }
    }

    fn resize(&mut self, content_length: usize) {
        self.state.select(Some(0));
        self.scroll = self.scroll.content_length(content_length);

        self.content_length = content_length;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum KeyViewState {
    Base,
    Subkey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LastSelected {
    Keys,
    Values,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ViewState {
    Keys,
    Values,
    Input(LastSelected),
    Message(LastSelected),
}

impl From<ViewState> for LastSelected {
    fn from(value: ViewState) -> Self {
        match value {
            ViewState::Keys => LastSelected::Keys,
            ViewState::Values => LastSelected::Values,
            _ => LastSelected::None,
        }
    }
}

impl From<LastSelected> for ViewState {
    fn from(value: LastSelected) -> Self {
        match value {
            LastSelected::Keys => ViewState::Keys,
            LastSelected::Values => ViewState::Values,
            LastSelected::None => ViewState::Keys, // default to keys
        }
    }
}

impl ViewState {
    pub const fn is_input(&self) -> bool {
        match self {
            Self::Input(_) => true,
            _ => false,
        }
    }

    pub const fn is_message(&self) -> bool {
        match self {
            Self::Message(_) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NamedValue {
    pub name: String,
    pub value: windows_registry::Value,
}

impl NamedValue {
    pub const fn new(name: String, value: windows_registry::Value) -> Self {
        Self { name, value }
    }
}

struct KeyState {
    key: windows_registry::Key,
    subkeys: Vec<String>,

    path_cache: String,
    values_cache: HashMap<String, Vec<NamedValue>>,
}

impl KeyState {
    fn new(key: windows_registry::Key, name: String, subkeys: Vec<String>, last_path: String) -> Self {
        let new_path = format!("{last_path} -> {name}");

        Self { key, subkeys, path_cache: new_path, values_cache: HashMap::new() }
    }
}

pub struct AppContext {
    pub key_table: ScrollableTableState,
    pub value_table: ScrollableTableState,
    pub input: InputState,
    pub message: Option<AppMessage>,
    pub view_state: ViewState,

    base_subkeys: Vec<String>,
    base_path: &'static str,
    key_states: Vec<KeyState>,
}

impl AppContext {
    pub fn new() -> Self {
        let base_subkeys: Vec<String> = Vec::from(registry::get_default_keys().map(|(_, name)| name.into()));

        Self {
            key_table: ScrollableTableState::new(base_subkeys.len() * ITEM_HEIGHT),
            value_table: ScrollableTableState::new(100 * ITEM_HEIGHT),
            input: InputState::new(),
            message: None,
            view_state: ViewState::Keys,

            base_subkeys,
            base_path: "Computer",

            key_states: Vec::new(),
        }
    }

    pub const fn get_selected_table(&mut self) -> Option<&mut ScrollableTableState> {
        match self.view_state {
            ViewState::Keys => Some(&mut self.key_table),
            ViewState::Values => Some(&mut self.value_table),
            _ => None,
        }
    }

    pub fn switch_views(&mut self) {
        self.view_state = match self.view_state {
            ViewState::Keys => ViewState::Values,
            ViewState::Values => ViewState::Keys,
            _ => return,
        };
    }

    fn update_values(&mut self) {
        let i = match self.key_table.state.selected() {
            Some(i) => i,
            None => return,
        };

        let key_name = self.get_subkeys()[i].clone();
        let key_state = match self.key_states.last_mut() {
            Some(key_state) => key_state,
            None => return,
        };

        let entry = key_state.values_cache.entry(key_name.clone()).or_insert_with(|| {
            let key = match registry::read_key(&key_state.key, key_name.as_str()) {
                Ok(key) => key,
                Err(_) => return Vec::new(),
            };

            let values = registry::read_values(&key);
            
            match values {
                Ok(values) => values.into_iter().map(|(name, value)| NamedValue::new(name, value)).collect(),
                Err(_) => Vec::new(),
            }
        });

        self.value_table.resize(entry.len() * ITEM_HEIGHT);
    }

    pub fn get_values(&self) -> Option<&Vec<NamedValue>> {
        let i = match self.key_table.state.selected() {
            Some(i) => i,
            None => return None,
        };

        let key_name = &self.get_subkeys()[i];
        let key_state = match self.key_states.last() {
            Some(key_state) => key_state,
            None => return None,
        };

        key_state.values_cache.get(key_name)
    }

    fn get_current_view_max(&self) -> usize {
        let len = match self.view_state {
            ViewState::Keys => self.get_subkeys().len(),
            ViewState::Values => self.get_values().map_or(0, |values| values.len()),
            _ => 0,
        };

        len.saturating_sub(1)
    }

    fn update_row_selection(&mut self, i: usize) {
        let table = match self.get_selected_table() {
            Some(table) => table,
            None => return,
        };

        table.state.select(Some(i));
        table.scroll = table.scroll.position(i * ITEM_HEIGHT);

        if self.view_state == ViewState::Keys {
            self.update_values();
        }
    }

    pub fn next_row(&mut self) {
        let max = self.get_current_view_max();
        let table = match self.get_selected_table() {
            Some(table) => table,
            None => return,
        };
        let i = table.state.selected().map_or(0, |i| i.saturating_add(1).min(max));

        self.update_row_selection(i);
    }

    pub fn prev_row(&mut self) {
        let table = match self.get_selected_table() {
            Some(table) => table,
            None => return,
        };
        let i = table.state.selected().map_or(0, |i| i.saturating_sub(1));

        self.update_row_selection(i);
    }

    fn get_key_view_state(&self) -> KeyViewState {
        match self.key_states.is_empty() {
            true => KeyViewState::Base,
            false => KeyViewState::Subkey,
        }
    }

    fn create_subkeys(&self, key: &windows_registry::Key) -> Vec<String> {
        let mut subkeys = registry::read_subkeys(key).unwrap();

        // add subkey to go back
        subkeys.insert(0, "..".into());

        subkeys
    }

    fn select_base(&mut self, index: usize) {
        let path = &self.base_subkeys[index];

        let default = registry::get_default_keys();
        let (key, name) = default.iter().find(|(_, s)| *s == path).unwrap();

        let subkeys = self.create_subkeys(key);
        let key = key.open("").unwrap();
        let new_state = KeyState::new(key, String::from(*name), subkeys, self.base_path.to_owned());

        self.key_states.push(new_state);
    }

    fn select_key(&mut self, index: usize) {
        match index {
            0 => { // ".." subkey
                let _ = self.key_states.pop();
            }
            _ => {
                let path = &self.get_subkeys()[index];
                let current_state = self.key_states.last().unwrap();

                let key = registry::read_key(&current_state.key, path).unwrap();
                let subkeys = self.create_subkeys(&key);
                let new_state = KeyState::new(key, path.to_owned(), subkeys, current_state.path_cache.clone());

                self.key_states.push(new_state);
            }
        };
    }

    pub fn select(&mut self) {
        let i = match self.key_table.state.selected() {
            Some(i) => i,
            None => return,
        };

        match self.get_key_view_state() {
            KeyViewState::Base => self.select_base(i),
            KeyViewState::Subkey => self.select_key(i),
        };

        self.key_table.resize(self.get_subkeys().len() * ITEM_HEIGHT);
    }

    pub fn get_path(&self) -> &str {
        match self.get_key_view_state() {
            KeyViewState::Base => self.base_path,
            KeyViewState::Subkey => self.key_states.last().unwrap().path_cache.as_str(),
        }
    }

    pub fn get_subkeys(&self) -> &Vec<String> {
        match self.get_key_view_state() {
            KeyViewState::Base => &self.base_subkeys,
            KeyViewState::Subkey => &self.key_states.last().unwrap().subkeys,
        }
    }

    pub fn set_input(&mut self) {
        self.view_state = ViewState::Input(self.view_state.into());
        self.input.confirm = false;
    }

    pub fn set_confirm_input(&mut self, validate: Box<InputValidateFn>, confirm: Box<InputConfirmFn>) {
        self.view_state = ViewState::Input(self.view_state.into());
        self.input.confirm = true;
        self.input.validate_fn = Some(validate);
        self.input.confirm_fn = Some(confirm);

        self.input.textarea.set_placeholder_text("<Esc> to cancel, <Enter> to confirm");
    }

    fn reset_input(&mut self) {
        self.input.confirm = false;
        self.input.validate_fn = None;
        self.input.confirm_fn = None;

        self.input.textarea.select_all();
        self.input.textarea.cut();

        self.input.textarea.set_placeholder_text("");

        self.input.label = "No Input Required".into();
    }

    pub fn escape_input(&mut self) {
        self.view_state = match self.view_state {
            ViewState::Input(last_selected) => last_selected.into(),
            _ => self.view_state,
        };

        self.reset_input();
    }

    pub fn confirm_input(&mut self) {
        if !self.input.confirm || self.input.validate().is_some_and(|res| res.is_err()) {
            return;
        }

        let text = self.input.text();

        match self.input.confirm_fn.as_ref() {
            Some(confirm_fn) => {
                let result = (confirm_fn)(text);
                let last_selected = match self.view_state {
                    ViewState::Input(last_selected) => last_selected,
                    _ => LastSelected::None,
                };

                result.map(|result| self.set_message_with_state(result, last_selected));
            }
            None => (),
        };

        self.reset_input();
    }

    pub fn set_message_with_state(&mut self, message: AppMessage, last_selected: LastSelected) {
        self.view_state = ViewState::Message(last_selected);
        self.message = Some(message);
    }

    pub fn set_message(&mut self, message: AppMessage) {
        self.set_message_with_state(message, self.view_state.into());
    }

    pub fn cancel_message(&mut self) {
        self.view_state = match self.view_state {
            ViewState::Message(last_selected) => last_selected.into(),
            _ => self.view_state,
        };

        self.message = None;
    }

    pub fn create(&mut self) {
        let validate = |text: &str| {
            if text.trim().is_empty() {
                return Err("Can't be empty".into());
            }

            Ok(())
        };

        let confirm = |text: String| {
            if text.to_lowercase() == "error" {
                Some(AppMessage::error("Confirmation Test: 'Create' error message test."))
            } else {
                Some(AppMessage::info("Confirmation Test: 'Create' confirmed."))
            }
        };

        self.input.label = "Enter Name:".into();
        self.set_confirm_input(Box::new(validate), Box::new(confirm));
    }

    pub fn rename(&mut self) {
        todo!()
    }

    pub fn delete(&mut self) {
        todo!()
    }

    pub fn change_type(&mut self) {
        todo!()
    }

    pub fn change_data(&mut self) {
        todo!()
    }
}
