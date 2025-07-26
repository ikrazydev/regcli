use std::{collections::HashMap, usize};

use ratatui::widgets::{ScrollbarState, TableState};
use tui_textarea::TextArea;

use crate::{app::ITEM_HEIGHT, registry};

pub type InputValidateFn = dyn Fn(&str) -> Result<(), String>;
pub type InputConfirmFn = dyn Fn(String) -> (Option<AppMessage>, PostAction);

pub struct InputChoices {
    pub items: Vec<String>,
    pub selected: usize,
}

impl InputChoices {
    pub fn new(items: Vec<impl Into<String>>) -> Self {
        assert!(items.len() > 0);

        Self {
            items: items.into_iter().map(|s| s.into()).collect(),
            selected: 0,
        }
    }
}

pub enum InputType {
    TextArea,
    Choice(InputChoices),
}

impl InputType {
    pub const fn is_textarea(&self) -> bool {
        match self {
            Self::TextArea => true,
            _ => false,
        }
    }

    pub const fn is_choice(&self) -> bool {
        !self.is_textarea()
    }
}

pub struct InputState {
    pub label: String,
    pub textarea: TextArea<'static>,

    pub validate_fn: Option<Box<InputValidateFn>>,
    pub confirm_fn: Option<Box<InputConfirmFn>>,

    pub ty: InputType,
}

impl InputState {
    fn new() -> Self {
        Self { label: String::from("No Input Required"), textarea: TextArea::default(), validate_fn: None, confirm_fn: None, ty: InputType::TextArea }
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

pub struct ActionAddSubkey {
    pub name: String,
}

pub struct ActionRenameSubkey {
    pub original: String,
    pub new: String,
}

pub struct ActionDeleteSubkey {
    pub name: String,
}

pub struct StageNewValueType {
    pub name: String,
}

pub struct StageNewValueData {
    pub name: String,
    pub ty: windows_registry::Type,
}

pub enum InputStageType {
    NewValueType(StageNewValueType),
    NewValueData(StageNewValueData),
}

pub struct ActionStage {
    pub ty: InputStageType,
}

pub enum PostAction {
    AddSubkey(ActionAddSubkey),
    RenameSubkey(ActionRenameSubkey),
    DeleteSubkey(ActionDeleteSubkey),

    Stage(ActionStage),

    None,
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
            _ => unreachable!(),
        }
    }
}

impl From<LastSelected> for ViewState {
    fn from(value: LastSelected) -> Self {
        match value {
            LastSelected::Keys => ViewState::Keys,
            LastSelected::Values => ViewState::Values,
            _ => unreachable!(),
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

    cached_path: String,
    cached_values: HashMap<String, Vec<NamedValue>>,
}

impl KeyState {
    fn new(key: windows_registry::Key, name: String, subkeys: Vec<String>, last_path: String) -> Self {
        let new_path = format!("{last_path} -> {name}");

        Self { key, subkeys, cached_path: new_path, cached_values: HashMap::new() }
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
        let base_subkeys: Vec<String> = Vec::from(registry::DEFAULT_KEYS.map(|(_, name)| name.into()));

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

    pub const fn get_table_by_view(&mut self, view: ViewState) -> Option<&mut ScrollableTableState> {
        match view {
            ViewState::Keys => Some(&mut self.key_table),
            ViewState::Values => Some(&mut self.value_table),
            _ => None,
        }
    }

    pub const fn get_selected_table(&mut self) -> Option<&mut ScrollableTableState> {
        self.get_table_by_view(self.view_state)
    }

    pub fn swap_viewing_table(&mut self) {
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

        let entry = key_state.cached_values.entry(key_name.clone()).or_insert_with(|| {
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

        key_state.cached_values.get(key_name)
    }

    fn get_current_view_max(&self) -> usize {
        let len = match self.view_state {
            ViewState::Keys => self.get_subkeys().len(),
            ViewState::Values => self.get_values().map_or(0, |values| values.len()),
            _ => 0,
        };

        len.saturating_sub(1)
    }

    fn select_row_in(&mut self, view: ViewState, i: usize) {
        let Some(table) = self.get_table_by_view(view) else { return; };

        table.state.select(Some(i));
        table.scroll = table.scroll.position(i * ITEM_HEIGHT);

        if self.view_state == ViewState::Keys {
            self.update_values();
        }
    }

    fn select_row_in_current(&mut self, i: usize) {
        self.select_row_in(self.view_state, i);
    }

    pub fn next_row(&mut self) {
        let max = self.get_current_view_max();
        let table = match self.get_selected_table() {
            Some(table) => table,
            None => return,
        };
        let i = table.state.selected().map_or(0, |i| i.saturating_add(1).min(max));

        self.select_row_in_current(i);
    }

    pub fn prev_row(&mut self) {
        let table = match self.get_selected_table() {
            Some(table) => table,
            None => return,
        };
        let i = table.state.selected().map_or(0, |i| i.saturating_sub(1));

        self.select_row_in_current(i);
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

        let default = registry::DEFAULT_KEYS;
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
                let new_state = KeyState::new(key, path.to_owned(), subkeys, current_state.cached_path.clone());

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
            KeyViewState::Subkey => self.key_states.last().unwrap().cached_path.as_str(),
        }
    }

    pub fn get_subkeys(&self) -> &Vec<String> {
        match self.get_key_view_state() {
            KeyViewState::Base => &self.base_subkeys,
            KeyViewState::Subkey => &self.key_states.last().unwrap().subkeys,
        }
    }

    fn set_input_state(&mut self, ty: InputType, validate_fn: Option<Box<InputValidateFn>>, confirm_fn: Option<Box<InputConfirmFn>>) {
        self.view_state = match self.view_state {
            ViewState::Input(_) => self.view_state,
            _ => ViewState::Input(self.view_state.into()),
        };

        self.input.ty = ty;
        self.input.validate_fn = validate_fn;
        self.input.confirm_fn = confirm_fn;
    }

    pub fn set_textarea_input(&mut self, validate: Box<InputValidateFn>, confirm: Box<InputConfirmFn>) {
        self.set_input_state(InputType::TextArea, Some(validate), Some(confirm));

        self.input.textarea.select_all();
        self.input.textarea.cut();

        self.input.textarea.set_placeholder_text("<Esc> to cancel, <Enter> to confirm");
    }

    pub fn set_choice_input(&mut self, choices: Vec<impl Into<String>>, confirm: Box<InputConfirmFn>) {
        self.set_input_state(InputType::Choice(InputChoices::new(choices)), None, Some(confirm));
    }

    pub fn next_input_choice(&mut self) {
        let InputType::Choice(ref mut choices) = self.input.ty else { return; };
        let len = choices.items.len();

        choices.selected = (choices.selected + 1) % len;
    }

    pub fn prev_input_choice(&mut self) {
        let InputType::Choice(ref mut choices) = self.input.ty else { return; };
        let len = choices.items.len();

        choices.selected = (choices.selected + len - 1) % len;
    }

    pub fn reset_input(&mut self) {
        self.view_state = match self.view_state {
            ViewState::Input(last_selected) => last_selected.into(),
            _ => self.view_state,
        };

        self.input.validate_fn = None;
        self.input.confirm_fn = None;

        if self.input.ty.is_textarea() {
            self.input.textarea.select_all();
            self.input.textarea.cut();

            self.input.textarea.set_placeholder_text("");
        }

        self.input.label = "No Input Required".into();
    }

    fn post_action_add_subkey(&mut self, action: ActionAddSubkey) {
        let Some(last) = self.key_states.last_mut() else { unreachable!() };
        let Err(index) = last.subkeys.binary_search_by(|s| s.cmp(&action.name)) else { unreachable!() };

        last.subkeys.insert(index, action.name);

        self.key_table.resize(last.subkeys.len() * ITEM_HEIGHT);
        self.select_row_in(ViewState::Keys, index);
    }

    fn post_action_rename_subkey(&mut self, action: ActionRenameSubkey) {
        let Some(last) = self.key_states.last_mut() else { unreachable!() };
        let Some(subkey_index) = last.subkeys.iter().position(|a| *a == action.original) else { unreachable!() };

        last.subkeys[subkey_index] = action.new;
    }

    fn post_action_delete_subkey(&mut self, action: ActionDeleteSubkey) {
        let Some(last) = self.key_states.last_mut() else { unreachable!() };
        let Ok(index) = last.subkeys.binary_search_by(|s| s.cmp(&action.name)) else { unreachable!() };

        last.subkeys.remove(index);

        self.key_table.resize(last.subkeys.len() * ITEM_HEIGHT);
        self.select_row_in(ViewState::Keys, index - 1);
    }

    fn input_stage_new_value_type(&mut self, stage: StageNewValueType) {
        let confirm = move |input: String| {
            let ty = registry::str_to_type(input.as_ref());

            (
                None,
                PostAction::Stage(ActionStage {
                    ty: InputStageType::NewValueData(StageNewValueData { name: stage.name.clone(), ty }) 
                })
            )
        };

        self.input.label = "Choose Type:".into();
        self.set_choice_input(registry::get_type_choices_vec(), Box::new(confirm));
    }

    fn input_stage_new_value_data(&mut self, stage: StageNewValueData) {
        let validate = move |input: &str| {
            Ok(())
        };

        let confirm = move |input: String| {
            (None, PostAction::None)
        };

        self.input.label = "Enter Value:".into();
        self.set_textarea_input(Box::new(validate), Box::new(confirm));
    }

    fn post_action_stage(&mut self, action: ActionStage) {
        match action.ty {
            InputStageType::NewValueType(stage) => self.input_stage_new_value_type(stage),
            InputStageType::NewValueData(stage) => self.input_stage_new_value_data(stage),
        }
    }

    pub fn confirm_input(&mut self) {
        if self.input.validate().is_some_and(|res| res.is_err()) {
            return;
        }

        let text = match self.input.ty {
            InputType::TextArea => self.input.text(),
            InputType::Choice(ref choices) => choices.items[choices.selected].clone(),
        };

        let mut should_reset_input = true;

        match self.input.confirm_fn.as_ref() {
            Some(confirm_fn) => {
                let (message, action) = (confirm_fn)(text);
                let last_selected = match self.view_state {
                    ViewState::Input(last_selected) => last_selected,
                    _ => LastSelected::None,
                };

                message.map(|result| self.set_message_with_state(result, last_selected));

                match action {
                    PostAction::AddSubkey(action) => self.post_action_add_subkey(action),
                    PostAction::RenameSubkey(action) => self.post_action_rename_subkey(action),
                    PostAction::DeleteSubkey(action) => self.post_action_delete_subkey(action),

                    PostAction::Stage(action) => {
                        should_reset_input = false;
                        self.post_action_stage(action);
                    }

                    PostAction::None => (),
                };
            }
            None => (),
        };

        if should_reset_input {
            self.reset_input();
        }
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

    fn key_name_validator(input: &str, subkeys: &Vec<String>, exclude_keys: &Vec<String>) -> Result<(), String> {
        if input.trim().is_empty() {
            return Err("Can't be empty".into());
        }
        if input.contains('/') {
            return Err("Name of a key can't contain forward slashes".into());
        }
        if input.len() > 255 {
            return Err("Name of a key can't be longer than 255 characters".into());
        }

        let found_key = subkeys.iter().find(|&a| a.to_lowercase() == input.to_lowercase());
        let is_excluded = exclude_keys.iter().any(|a| *a.to_lowercase() == input.to_lowercase());
        if found_key.is_some() && !is_excluded {
            return Err("This key already exists".into());
        }

        Ok(())
    }

    fn value_name_validator(input: &str, values: &Vec<NamedValue>, exclude_values: &Vec<NamedValue>) -> Result<(), String> {
        if input.trim().is_empty() {
            return Err("Can't be empty".into());
        }
        if input.contains('/') {
            return Err("Name of a value can't contain forward slashes".into());
        }
        if input.len() > 16383 {
            return Err("Name of a value can't be longer than 16,383 characters".into());
        }

        let found_value = values.iter().find(|&a| a.name.to_lowercase() == input.to_lowercase());
        let is_excluded = exclude_values.iter().any(|a| *a.name.to_lowercase() == input.to_lowercase());
        if found_value.is_some() && !is_excluded {
            return Err("This value already exists".into());
        }

        Ok(())
    }

    fn get_selected_subkey(&self) -> Option<(&KeyState, &String)> {
        let selected = self.key_table.state.selected()?;
        let state = self.key_states.last()?;

        let subkey = state.subkeys.get(selected)?;

        Some((state, subkey))
    }

    pub fn new_key(&mut self) {
        let Some((key, subkeys)) = self.key_states.last()
            .map(|s| (registry::clone_key(&s.key), s.subkeys.clone()))
            else {
                self.set_message(AppMessage::error("Can't create a key here."));
                return;
            };

        let exclude = Vec::new();
        let validate = move |input: &str| { Self::key_name_validator(input, &subkeys, &exclude) };

        let confirm = move |input: String| {
            match registry::new_key(&key, input.as_str()) {
                Ok(()) => {
                    (Some(AppMessage::info("New key successfully created.")), PostAction::AddSubkey(ActionAddSubkey { name: input }))
                }
                Err(err) => {
                    (Some(AppMessage::error(format!("Error when creating a new key: {}", err.message()))), PostAction::None)
                }
            }
        };

        self.input.label = "Enter Name:".into();
        self.set_textarea_input(Box::new(validate), Box::new(confirm));
    }

    pub fn new_value(&mut self) {
        let (state, key) = match self.get_selected_subkey() {
            Some((state, key)) => (state, key),
            None => {
                self.set_message(AppMessage::info("No key selected."));
                return;
            } 
        };

        let Some(values) = state.cached_values.get(key).cloned() else { unreachable!() };

        let exclude = Vec::new();
        let validate = move |input: &str| { Self::value_name_validator(input, &values, &exclude) };

        let confirm = move |input: String| {
            (
                None,
                PostAction::Stage(ActionStage {
                    ty: InputStageType::NewValueType(StageNewValueType { name: input }) 
                })
            )
        };

        self.input.label = "Enter Name:".into();
        self.set_textarea_input(Box::new(validate), Box::new(confirm));
    }

    fn truncate_name(s: impl AsRef<str>, max_len: usize, sides_size: usize) -> String {
        assert!(max_len >= sides_size * 2);

        let s = s.as_ref();
        let len = s.len();

        if len <= max_len {
            return s.into();
        }

        let left = s.chars().take(sides_size).collect::<String>();
        let right = s.chars().skip(len - sides_size).collect::<String>();

        format!("{}...{}", left, right)
    }

    pub fn rename_key(&mut self) {
        let Some((key, subkeys)) = self.key_states.last()
        .map(|s| (registry::clone_key(&s.key), s.subkeys.clone()))
        else {
            self.set_message(AppMessage::error("Can't rename a key here."));
            return;
        };

        let selection = self.key_table.state.selected().unwrap_or(0);
        if selection == 0 {
            self.set_message(AppMessage::error("No key selected."));
            return;
        }

        let current_name = (&subkeys[selection]).to_owned();
        let short_name = Self::truncate_name(current_name.as_str(), 10, 3);

        let exclude = vec![current_name.clone()];
        let validate = move |input: &str| {
            if input == exclude[0] {
                return Err("The name of the key must be new".into());
            }
            Self::key_name_validator(input, &subkeys, &exclude)
        };

        let confirm = move |input: String| {
            match registry::rename_key(&key, current_name.as_str(), input.as_str()) {
                Ok(()) => {
                    (Some(AppMessage::info("The key has been successfully renamed.")), PostAction::RenameSubkey(ActionRenameSubkey { original: current_name.clone(), new: input }))
                }
                Err(err) => {
                    (Some(AppMessage::error(format!("Error when renaming the key: {}", err.message()))), PostAction::None)
                }
            }
        };

        self.input.label = format!("Enter New Name ({}):", short_name);
        self.set_textarea_input(Box::new(validate), Box::new(confirm));
    }

    pub fn rename_value(&mut self) {
        todo!()
    }

    pub fn delete_key(&mut self) {
        let Some((key, subkeys)) = self.key_states.last()
            .map(|s| (registry::clone_key(&s.key), s.subkeys.clone()))
            else {
                self.set_message(AppMessage::error("Can't delete a key here."));
                return;
            };

        let selection = self.key_table.state.selected().unwrap_or(0);
        if selection == 0 {
            self.set_message(AppMessage::error("No key selected."));
            return;
        }

        let current_name = (&subkeys[selection]).to_owned();

        let confirm = move |text: String| {
            if text == "No" {
                return (None, PostAction::None);
            }

            match registry::delete_key(&key, current_name.as_str()) {
                Ok(()) => {
                    (Some(AppMessage::info("The key has been successfully deleted.")), PostAction::DeleteSubkey(ActionDeleteSubkey { name: current_name.clone() }))
                }
                Err(err) => {
                    (Some(AppMessage::error(format!("Error when deleting the key: {}", err.message()))), PostAction::None)
                }
            }
        };

        self.input.label = "Confirm Delete:".into();
        self.set_choice_input(vec!["No", "Yes"], Box::new(confirm));
    }

    pub fn change_type(&mut self) {
        todo!()
    }

    pub fn change_data(&mut self) {
        todo!()
    }

    pub fn delete_value(&mut self) {
        todo!()
    }

    fn dispatch_by_view<F, G>(&mut self, on_keys: F, on_values: G)
    where
        F: FnOnce(&mut Self),
        G: FnOnce(&mut Self),
    {
        match self.view_state {
            ViewState::Keys => on_keys(self),
            ViewState::Values => on_values(self),
            _ => unreachable!(),
        }
    }

    pub fn create(&mut self) {
        self.dispatch_by_view(Self::new_key, Self::new_value);
    }

    pub fn rename(&mut self) {
        self.dispatch_by_view(Self::rename_key, Self::rename_value);
    }

    pub fn delete(&mut self) {
        self.dispatch_by_view(Self::delete_key, Self::delete_value);
    }
}
