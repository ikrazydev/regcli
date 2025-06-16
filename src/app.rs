use crossterm::event::{self, Event, KeyCode};
use ratatui::{layout::{Constraint, Layout, Margin, Rect}, prelude::Backend, style::{Style, Stylize}, text::{Line, Text}, widgets::{Block, Cell, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table, TableState}, Frame, Terminal};

const ITEM_HEIGHT: usize = 1;

struct TableContext {
    state: TableState,
    scroll_state: ScrollbarState,
    items: Vec<String>,
}

pub struct App {
    table_context: TableContext,
}

impl TableContext {
    fn new() -> Self {
        let items: Vec<String> = (0..100).map(|num| format!("Item {num}")).collect();

        Self {
            state: TableState::default().with_selected(0),
            scroll_state: ScrollbarState::new(items.len() * ITEM_HEIGHT),
            items,
        }
    }
}

impl App {
    pub fn new() -> Self {
        Self {
            table_context: TableContext::new(),
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
                _ => (),
            }
            _ => (),
        }

        Ok(false)
    }

    fn next_row(&mut self) {
        let i = match self.table_context.state.selected() {
            Some(i) => {
                if i >= self.table_context.items.len() - 1 {
                    i
                } else {
                    i + 1
                }
            }
            None => 0,
        };

        self.table_context.state.select(Some(i));
        self.table_context.scroll_state = self.table_context.scroll_state.position(i * ITEM_HEIGHT);
    }

    fn prev_row(&mut self) {
        let i = match self.table_context.state.selected() {
            Some(i) => {
                if i == 0 {
                    i
                } else {
                    i - 1
                }
            }
            None => 0,
        };

        self.table_context.state.select(Some(i));
        self.table_context.scroll_state = self.table_context.scroll_state.position(i * ITEM_HEIGHT);
    }

    fn render_table(&mut self, frame: &mut Frame, area: Rect) {
        let selected_style = Style::default()
            .black()
            .on_white();
        let header_style = Style::default()
            .white()
            .on_dark_gray()
            .bold();

        let header = ["Item"]
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .style(header_style)
            .height(1);

        let rows = self.table_context.items.iter().map(|item| {
            Row::new(vec![Cell::from(Text::from(item.to_owned()))]).height(ITEM_HEIGHT as u16)
        });
        let table = Table::new(rows, [Constraint::Min(0)])
            .header(header)
            .row_highlight_style(selected_style);

        frame.render_stateful_widget(table, area, &mut self.table_context.state);

        frame.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None),
            area.inner(Margin { vertical: 1, horizontal: 1 }),
            &mut self.table_context.scroll_state);
    }

    fn draw(&mut self, frame: &mut Frame) {
        use Constraint::{Length, Min};

        let vertical = Layout::vertical([Length(3), Min(0), Length(3)]);
        let [title_area, main_area, status_area] = vertical.areas(frame.area());

        let bold_style = Style::new().bold();

        let title_block = Block::bordered().title("Regedit").style(bold_style);
        let title_content = Paragraph::new("Computer -> Registry").block(title_block);
        frame.render_widget(title_content, title_area);

        self.render_table(frame, main_area.clone());

        let status = Line::from(vec![
            " <Esc> ".black().on_white().bold(),
            " Quit ".into(),
            " <J> ".black().on_white().bold(),
            " Item Down ".into(),
            " <K> ".black().on_white().bold(),
            " Item Up ".into(),
        ]);

        frame.render_widget(status, status_area);
    }
}
