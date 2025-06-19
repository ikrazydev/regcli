use crossterm::event::{self, Event, KeyCode};
use ratatui::{layout::{Constraint, Layout, Margin, Rect}, prelude::Backend, style::{Style, Stylize}, text::{Line, Text}, widgets::{Block, Cell, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table, TableState}, Frame, Terminal};

use crate::registry;

const ITEM_HEIGHT: usize = 1;

enum ViewState {
    Base,
    Subkey,
}

struct KeyState {
    key: windows_registry::Key,
    subkeys: Vec<String>,

    path_cache: String,
    // values_cache: HashMap<(), ()>, // for future use
}

impl KeyState {
    fn new(key: windows_registry::Key, name: String, subkeys: Vec<String>, last_path: String) -> Self {
        let new_path = format!("{last_path} -> {name}");

        Self { key, subkeys, path_cache: new_path }
    }
}

struct AppContext {
    key_table_state: TableState,
    key_scroll_state: ScrollbarState,

    base_subkeys: Vec<String>,
    base_path: String,
    key_states: Vec<KeyState>,
}

pub struct App {
    context: AppContext,
}

impl AppContext {
    fn new() -> Self {
        let base_subkeys: Vec<String> = Vec::from(registry::get_default_keys().map(|(_, name)| name.into()));

        Self {
            key_table_state: TableState::default().with_selected(0),
            key_scroll_state: ScrollbarState::new(base_subkeys.len() * ITEM_HEIGHT),

            base_subkeys,
            base_path: String::from("Computer"),

            key_states: Vec::new(),
        }
    }

    fn get_view_state(&self) -> ViewState {
        match self.key_states.is_empty() {
            true => ViewState::Base,
            false => ViewState::Subkey,
        }
    }

    fn create_subkeys(&self, key: &windows_registry::Key) -> Vec<String> {
        let mut subkeys = registry::read_subkeys(key);

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
        let new_state = KeyState::new(key, String::from(*name), subkeys, self.base_path.clone());

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

                let key = registry::read_key(&current_state.key, path);
                let subkeys = self.create_subkeys(&key);
                let new_state = KeyState::new(key, path.to_owned(), subkeys, current_state.path_cache.clone());

                self.key_states.push(new_state);
            }
        };
    }

    fn select(&mut self) {
        let i = self.key_table_state.selected();
        if i.is_none() {
            return;
        }

        let i = i.unwrap();

        match self.get_view_state() {
            ViewState::Base => self.select_base(i),
            ViewState::Subkey => self.select_key(i),
        };
    }

    fn get_path(&self) -> &str {
        match self.get_view_state() {
            ViewState::Base => self.base_path.as_str(),
            ViewState::Subkey => self.key_states.last().unwrap().path_cache.as_str(),
        }
    }

    fn get_subkeys(&self) -> &Vec<String> {
        match self.get_view_state() {
            ViewState::Base => &self.base_subkeys,
            ViewState::Subkey => &self.key_states.last().unwrap().subkeys,
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
                KeyCode::Char('j') => self.next_row(),
                KeyCode::Char('k') => self.prev_row(),
                KeyCode::Enter => self.select(),
                _ => (),
            }
            _ => (),
        }

        Ok(false)
    }

    fn next_row(&mut self) {
        let i = match self.context.key_table_state.selected() {
            Some(i) => {
                if i >= self.context.get_subkeys().len() - 1 {
                    i
                } else {
                    i + 1
                }
            }
            None => 0,
        };

        self.context.key_table_state.select(Some(i));
        self.context.key_scroll_state = self.context.key_scroll_state.position(i * ITEM_HEIGHT);
    }

    fn prev_row(&mut self) {
        let i = match self.context.key_table_state.selected() {
            Some(i) => {
                if i == 0 {
                    i
                } else {
                    i - 1
                }
            }
            None => 0,
        };

        self.context.key_table_state.select(Some(i));
        self.context.key_scroll_state = self.context.key_scroll_state.position(i * ITEM_HEIGHT);
    }

    fn select(&mut self) {
        self.context.select();
    }

    fn render_table(&mut self, frame: &mut Frame, area: Rect) {
        let selected_style = Style::default()
            .black()
            .on_white();
        let header_style = Style::default()
            .white()
            .on_dark_gray()
            .bold();

        let block = Block::bordered();

        let header = ["Registry Key"]
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .style(header_style)
            .height(1);

        let rows = self.context.get_subkeys().iter().map(|item| {
            Row::new(vec![Cell::from(Text::from(item.to_owned()))]).height(ITEM_HEIGHT as u16)
        });
        let table = Table::new(rows, [Constraint::Min(0)])
            .header(header)
            .row_highlight_style(selected_style)
            .block(block);

        frame.render_stateful_widget(table, area, &mut self.context.key_table_state);

        frame.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None),
            area.inner(Margin { vertical: 1, horizontal: 1 }),
            &mut self.context.key_scroll_state);
    }

    fn draw(&mut self, frame: &mut Frame) {
        use Constraint::{Length, Min};

        let vertical = Layout::vertical([Length(3), Min(0), Length(3)]);
        let [title_area, main_area, status_area] = vertical.areas(frame.area());

        let bold_style = Style::new().bold();

        let title_block = Block::bordered().title("Regedit").style(bold_style);
        let title_content = Paragraph::new(self.context.get_path()).block(title_block);
        frame.render_widget(title_content, title_area);

        self.render_table(frame, main_area.clone());

        let status = Line::from(vec![
            " <Esc> ".black().on_white().bold(),
            " Quit ".into(),
            " <J> ".black().on_white().bold(),
            " Down ".into(),
            " <K> ".black().on_white().bold(),
            " Up ".into(),
            " <Enter> ".black().on_white().bold(),
            " Select ".into(),
        ]);

        frame.render_widget(status, status_area);
    }
}
