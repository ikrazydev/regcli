use crossterm::event::{self, Event, KeyCode};
use ratatui::{layout::{Constraint, Layout, Margin, Rect}, prelude::Backend, style::{Style, Stylize}, text::{Line, Text}, widgets::{Block, Cell, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table, TableState}, Frame, Terminal};

use crate::registry;

const ITEM_HEIGHT: usize = 1;

struct RegistryContext {
    state: TableState,
    scroll_state: ScrollbarState,
    items: Vec<String>,

    current_base_key: Option<&'static windows_registry::Key>,
    current_key: Option<windows_registry::Key>,
    current_path: String,
}

pub struct App {
    context: RegistryContext,
}

impl RegistryContext {
    fn new() -> Self {
        let items: Vec<String> = Vec::from(registry::get_default_keys().map(|(_, name)| name.into()));

        Self {
            state: TableState::default().with_selected(0),
            scroll_state: ScrollbarState::new(items.len() * ITEM_HEIGHT),
            items,

            current_base_key: None,
            current_key: None,
            current_path: String::from("Computer"),
        }
    }

    fn select_base(&mut self, path: &str) {
        if self.current_base_key.is_some() || self.current_key.is_some() {
            return;
        }

        let default = registry::get_default_keys();
        let (key, _) = default.iter().find(|(_, s)| *s == path).unwrap();

        self.items = registry::read_subkeys(*key);
        self.current_base_key = Some(key);
    }

    fn select_key(&mut self, path: &str) {
        if self.current_base_key.is_none() {
            return;
        }

        self.current_key = match self.current_key.take() {
            Some(key) => Some(registry::read_key(&key, path)),
            None => Some(registry::read_key(self.current_base_key.unwrap(), path)),
        };

        self.items = registry::read_subkeys(self.current_key.as_ref().unwrap());
    }

    fn select(&mut self) {
        let i = self.state.selected();
        if i.is_none() {
            return;
        }

        let i = i.unwrap();
        let path = self.items.get(i).unwrap().to_owned();

        match self.current_base_key {
            Some(_) => self.select_key(path.as_str()),
            None => self.select_base(path.as_str()),
        }

        self.current_path.push_str(" -> ");
        self.current_path.push_str(path.as_str());
    }
}

impl App {
    pub fn new() -> Self {
        Self {
            context: RegistryContext::new(),
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
        let i = match self.context.state.selected() {
            Some(i) => {
                if i >= self.context.items.len() - 1 {
                    i
                } else {
                    i + 1
                }
            }
            None => 0,
        };

        self.context.state.select(Some(i));
        self.context.scroll_state = self.context.scroll_state.position(i * ITEM_HEIGHT);
    }

    fn prev_row(&mut self) {
        let i = match self.context.state.selected() {
            Some(i) => {
                if i == 0 {
                    i
                } else {
                    i - 1
                }
            }
            None => 0,
        };

        self.context.state.select(Some(i));
        self.context.scroll_state = self.context.scroll_state.position(i * ITEM_HEIGHT);
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

        let rows = self.context.items.iter().map(|item| {
            Row::new(vec![Cell::from(Text::from(item.to_owned()))]).height(ITEM_HEIGHT as u16)
        });
        let table = Table::new(rows, [Constraint::Min(0)])
            .header(header)
            .row_highlight_style(selected_style)
            .block(block);


        frame.render_stateful_widget(table, area, &mut self.context.state);

        frame.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None),
            area.inner(Margin { vertical: 1, horizontal: 1 }),
            &mut self.context.scroll_state);
    }

    fn draw(&mut self, frame: &mut Frame) {
        use Constraint::{Length, Min};

        let vertical = Layout::vertical([Length(3), Min(0), Length(3)]);
        let [title_area, main_area, status_area] = vertical.areas(frame.area());

        let bold_style = Style::new().bold();

        let title_block = Block::bordered().title("Regedit").style(bold_style);
        let title_content = Paragraph::new(self.context.current_path.as_str()).block(title_block);
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
