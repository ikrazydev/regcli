use std::collections::HashMap;

use crossterm::event::{self, Event, KeyCode};
use ratatui::{layout::{Constraint, Layout, Margin, Rect}, prelude::Backend, style::{Style, Stylize}, text::{Line, Span}, widgets::{Block, Cell, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table, TableState}, Frame, Terminal};

use crate::registry;

const ITEM_HEIGHT: usize = 1;

struct ScrollableTableState {
    state: TableState,
    scroll: ScrollbarState,
    len: usize,
}

impl ScrollableTableState {
    fn new(content_length: usize) -> Self {
        Self {
            state: TableState::default().with_selected(0),
            scroll: ScrollbarState::new(content_length),
            len: content_length,
        }
    }

    fn resize(&mut self, content_length: usize) {
        self.state.select(Some(0));
        self.scroll = self.scroll.content_length(content_length);

        self.len = content_length;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum KeyViewState {
    Base,
    Subkey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ViewState {
    Keys,
    Values
}

#[derive(Debug, Clone)]
struct NamedValue {
    name: String,
    value: windows_registry::Value,
}

impl NamedValue {
    const fn new(name: String, value: windows_registry::Value) -> Self {
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

struct AppContext {
    key_table: ScrollableTableState,
    value_table: ScrollableTableState,
    view_state: ViewState,

    base_subkeys: Vec<String>,
    base_path: &'static str,
    key_states: Vec<KeyState>,
}

pub struct App {
    context: AppContext,
}

impl AppContext {
    fn new() -> Self {
        let base_subkeys: Vec<String> = Vec::from(registry::get_default_keys().map(|(_, name)| name.into()));

        Self {
            key_table: ScrollableTableState::new(base_subkeys.len() * ITEM_HEIGHT),
            value_table: ScrollableTableState::new(100 * ITEM_HEIGHT),
            view_state: ViewState::Keys,

            base_subkeys,
            base_path: "Computer",

            key_states: Vec::new(),
        }
    }

    const fn get_selected_table(&mut self) -> &mut ScrollableTableState {
        match self.view_state {
            ViewState::Keys => &mut self.key_table,
            ViewState::Values => &mut self.value_table,
        }
    }

    const fn switch_views(&mut self) {
        self.view_state = match self.view_state {
            ViewState::Keys => ViewState::Values,
            ViewState::Values => ViewState::Keys,
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

    fn get_values(&self) -> Option<&Vec<NamedValue>> {
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

    fn get_current_view_len(&self) -> usize {
        match self.view_state {
            ViewState::Keys => self.get_subkeys().len(),
            ViewState::Values => self.get_values().map_or(0, |values| values.len()),
        }
    }

    fn next_row(&mut self) {
        let max = self.get_current_view_len();
        if max == usize::MIN {
            return;
        }

        let table = self.get_selected_table();
        
        let i = match table.state.selected() {
            Some(i) => {
                if i >= max - 1 {
                    i
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        
        table.state.select(Some(i));
        table.scroll = table.scroll.position(i * ITEM_HEIGHT);

        if self.view_state == ViewState::Keys {
            self.update_values();
        }
    }

    fn prev_row(&mut self) {
        let table = self.get_selected_table();
        
        let i = match table.state.selected() {
            Some(i) => {
                if i == 0 {
                    i
                } else {
                    i - 1
                }
            }
            None => 0,
        };

        table.state.select(Some(i));
        table.scroll = table.scroll.position(i * ITEM_HEIGHT);

        if self.view_state == ViewState::Keys {
            self.update_values();
        }
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

    fn select(&mut self) {
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

    fn get_path(&self) -> &str {
        match self.get_key_view_state() {
            KeyViewState::Base => self.base_path,
            KeyViewState::Subkey => self.key_states.last().unwrap().path_cache.as_str(),
        }
    }

    fn get_subkeys(&self) -> &Vec<String> {
        match self.get_key_view_state() {
            KeyViewState::Base => &self.base_subkeys,
            KeyViewState::Subkey => &self.key_states.last().unwrap().subkeys,
        }
    }
}

impl App {
    pub fn new() -> Self {
        Self {
            context: AppContext::new(),
        }
    }

    pub fn run<B: Backend>(&mut self, term: &mut Terminal<B>) -> std::io::Result<()> {
        loop {
            if self.handle_events()? {
                break;
            }

            term.draw(|frame| self.draw(frame))?;
        }

        Ok(())
    }

    fn handle_events(&mut self) -> std::io::Result<bool> {
        match event::read()? {
            Event::Key(event) if event.is_press() => match event.code {
                KeyCode::Esc => return Ok(true),
                KeyCode::Char('j') | KeyCode::Char('J') => self.context.next_row(),
                KeyCode::Char('k') | KeyCode::Char('K') => self.context.prev_row(),
                KeyCode::Tab => self.context.switch_views(),
                KeyCode::Enter => self.context.select(),
                _ => (),
            }
            _ => (),
        }

        Ok(false)
    }

    fn render_title(&mut self, frame: &mut Frame, area: Rect) {
        let bold_style = Style::new().bold();

        let title_block = Block::bordered().title("Regedit").style(bold_style);
        let title_content = Paragraph::new(self.context.get_path()).block(title_block);
        frame.render_widget(title_content, area);
    }

    fn render_subkey_table(&mut self, frame: &mut Frame, area: Rect) {
        let header = ["Key"];

        let subkeys = self.context.get_subkeys().clone();
        let rows = subkeys.into_iter().map(|item| {
            Row::new(vec![item])
                .height(ITEM_HEIGHT as u16)
        });

        let is_disabled = self.context.view_state == ViewState::Keys;
        Self::render_table(frame, header, rows, &mut self.context.key_table, is_disabled, area);
    }

    fn render_empty_values(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::bordered();
        let paragraph = Paragraph::new("No Values to Display")
            .centered()
            .block(block);

        frame.render_widget(paragraph, area);
    }

    fn render_value_table(&mut self, frame: &mut Frame, area: Rect) {
        let header = ["Name", "Type", "Value"];
        let values = match self.context.get_values().cloned() {
            Some(values) => values,
            None => {
                self.render_empty_values(frame, area);
                return;
            }
        };

        let rows = values.into_iter()
            .map(|v| {
                let name = v.name.clone();
                let ty = registry::get_printable_type(v.value.ty()).to_owned();
                let value = registry::get_printable_value(&v.value);

                Row::new(
                    vec![name, ty, value]
                )
            }
        );

        let is_disabled = self.context.view_state == ViewState::Values;
        Self::render_table(frame, header, rows, &mut self.context.value_table, is_disabled, area);
    }

    fn render_table<'a, const N: usize, R>(frame: &mut Frame, header: [&str; N], rows: R, table: &mut ScrollableTableState, is_disabled: bool, area: Rect)
    where
        R: IntoIterator,
        R::Item: Into<Row<'a>>
    {
        let enabled_style = Style::default()
            .black()
            .on_white();
        let disabled_style = Style::default()
            .black()
            .on_gray();
        let selected_style = match is_disabled {
            true => enabled_style,
            false => disabled_style,
        };

        let header_style = Style::default()
            .white()
            .on_dark_gray()
            .bold();

        let block = Block::bordered();

        let header = header
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .style(header_style)
            .height(1);

        let widget = Table::new(rows, [Constraint::Min(0)].repeat(N))
            .header(header)
            .row_highlight_style(selected_style)
            .block(block);

        frame.render_stateful_widget(widget, area, &mut table.state);

        let content_height = table.len as u16;
        let viewport_height = area.height;

        if content_height > viewport_height {
            let scrollbar = Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None);
            frame.render_stateful_widget(
                scrollbar,
                area.inner(Margin { horizontal: 1, vertical: 1 }),
                &mut table.scroll);
        }
    }

    fn render_main_area(&mut self, frame: &mut Frame, area: Rect) {
        use Constraint::{Percentage, Min};

        let layout = Layout::horizontal([Percentage(40), Min(0)]);
        let [subkey_area, value_area] = layout.areas(area);

        self.render_subkey_table(frame, subkey_area);
        self.render_value_table(frame, value_area);
    }

    fn get_additional_keybinds(&self) -> Vec<Span<'_>> {
        match self.context.view_state {
            ViewState::Keys => vec![
                " <Enter> ".black().on_light_cyan().bold(),
                " Select ".into(),
            ],
            ViewState::Values => vec![
                " <null> ".black().on_light_cyan().bold(),
                " TODO ".into(),
            ],
        }
    }

    fn render_status(&mut self, frame: &mut Frame, area: Rect) {
        let mut keybinds = vec![
            " <Esc> ".black().on_white().bold(),
            " Quit ".into(),
            " <J> ".black().on_white().bold(),
            " Down ".into(),
            " <K> ".black().on_white().bold(),
            " Up ".into(),
            " <Tab> ".black().on_white().bold(),
            " Switch Views ".into(),
        ];

        keybinds.append(&mut self.get_additional_keybinds());

        let status = Line::from(keybinds);

        frame.render_widget(status, area);
    }

    fn draw(&mut self, frame: &mut Frame) {
        use Constraint::{Length, Min};

        let layout = Layout::vertical([Length(3), Min(0), Length(2)]);
        let [title_area, main_area, status_area] = layout.areas(frame.area());

        self.render_title(frame, title_area);
        self.render_main_area(frame, main_area);
        self.render_status(frame, status_area);
    }
}
