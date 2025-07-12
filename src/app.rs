use ratatui::{crossterm::event::{self, Event, KeyCode, KeyEventKind}, layout::{Constraint, Layout, Margin, Rect}, prelude::Backend, style::{Style, Stylize}, text::{Line, Span}, widgets::{Block, Cell, Paragraph, Row, Scrollbar, ScrollbarOrientation, Table}, Frame, Terminal};

use crate::{context::{AppContext, AppMessageType, ScrollableTableState, ViewState}, registry};

pub const ITEM_HEIGHT: usize = 1;

pub struct App {
    context: AppContext,
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

    fn handle_input_events(&mut self) -> std::io::Result<()> {
        match event::read()? {
            Event::Key(event) => match event.code {
                KeyCode::Esc if event.kind == KeyEventKind::Press => self.context.escape_input(),
                KeyCode::Enter if event.kind == KeyEventKind::Press => self.context.confirm_input(),
                _ => { self.context.input.textarea.input(event); }
            }
            _ => (),
        };

        Ok(())
    }

    fn handle_message_input_events(&mut self) -> std::io::Result<()> {
        match event::read()? {
            Event::Key(event) if event.kind == KeyEventKind::Press => self.context.cancel_message(),
            _ => (),
        };

        Ok(())
    }

    fn handle_events(&mut self) -> std::io::Result<bool> {
        if self.context.view_state.is_input() {
            self.handle_input_events()?;
            return Ok(false);
        }
        if self.context.view_state.is_message() {
            self.handle_message_input_events()?;
            return Ok(false);
        }

        match event::read()? {
            Event::Key(event) if event.kind == KeyEventKind::Press => match event.code {
                KeyCode::Esc => return Ok(true),
                KeyCode::Char('j') | KeyCode::Char('J') => self.context.next_row(),
                KeyCode::Char('k') | KeyCode::Char('K') => self.context.prev_row(),
                KeyCode::Tab => self.context.switch_views(),
                
                KeyCode::Enter => self.context.select(),
                KeyCode::Char('n') | KeyCode::Char('N') => self.context.create(),
                KeyCode::Char('r') | KeyCode::Char('R') => self.context.rename(),
                KeyCode::Char('d') | KeyCode::Char('D') => self.context.delete(),

                KeyCode::Char('t') | KeyCode::Char('T') => self.context.change_type(),
                KeyCode::Char('v') | KeyCode::Char('V') => self.context.change_data(),

                _ => (),
            }
            _ => (),
        }

        Ok(false)
    }

    fn render_title(&mut self, frame: &mut Frame, area: Rect) {
        let bold_style = Style::new().bold();

        let title_block = Block::bordered().title("Regcli").style(bold_style);
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
        let header = ["Name", "Type", "Data"];
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

        if rows.len() == 0 {
            self.render_empty_values(frame, area);
            return;
        }

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

        let content_height = table.content_length as u16;
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
                " Open ".into(),
                " <N> ".black().on_light_cyan().bold(),
                " New ".into(),
                " <R> ".black().on_light_cyan().bold(),
                " Rename ".into(),
                " <D> ".black().on_light_cyan().bold(),
                " Delete ".into(),
            ],
            ViewState::Values => vec![
                " <N> ".black().on_light_cyan().bold(),
                " New ".into(),
                " <R> ".black().on_light_cyan().bold(),
                " Rename ".into(),
                " <T> ".black().on_light_cyan().bold(),
                " Change Type ".into(),
                " <V> ".black().on_light_cyan().bold(),
                " Change Data ".into(),
                " <D> ".black().on_light_cyan().bold(),
                " Delete ".into(),
            ],
            _ => Vec::new(),
        }
    }

    fn render_message(&mut self, frame: &mut Frame, area: Rect) {
        let message = match self.context.message.as_ref() {
            Some(message) => message,
            None => return,
        };

        let text = message.message.clone();
        let text = text + " (Press any key to continue)";

        let style = match message.ty {
            AppMessageType::Info => Style::default().black().on_dark_gray(),
            AppMessageType::Error => Style::default().white().on_red(),
        };

        let label = Paragraph::new(text)
            .style(style)
            .block(Block::bordered().style(style));

        frame.render_widget(label, area);
    }

    fn render_input(&mut self, frame: &mut Frame, area: Rect) {
        let label_padding = 2;
        let label_text = self.context.input.label.as_str();

        let layout = Layout::horizontal([Constraint::Length(label_text.len() as u16 + label_padding), Constraint::Min(0)]);
        let [label_area, input_area] = layout.areas(area);

        let style = match self.context.view_state {
            ViewState::Input(_) => Style::default(),
            _ => Style::default().dark_gray(),
        };

        let label = Paragraph::new(label_text)
            .block(Block::bordered().style(style))
            .style(style);

        let block = match self.context.input.validate() {
            Some(Err(message)) => Block::bordered().red().title(message),
            Some(Ok(())) | None => Block::bordered().style(style),
        };

        self.context.input.textarea.set_style(style);
        self.context.input.textarea.set_block(block);

        match self.context.view_state {
            ViewState::Input(_) => {
                self.context.input.textarea.set_cursor_style(Style::default().on_white());
                self.context.input.textarea.set_cursor_line_style(Style::default().underlined());
            }
            _ => {
                self.context.input.textarea.set_cursor_style(Style::default());
                self.context.input.textarea.set_cursor_line_style(Style::default());
            }
        };

        frame.render_widget(label, label_area);
        frame.render_widget(&self.context.input.textarea, input_area);
    }

    fn render_footer(&mut self, frame: &mut Frame, area: Rect) {
        match self.context.view_state {
            ViewState::Message(_) => self.render_message(frame, area),
            _ => self.render_input(frame, area),
        };
    }

    fn render_status(&mut self, frame: &mut Frame, area: Rect) {
        use Constraint::{Min};

        let layout = Layout::vertical([Min(1), Min(3)]);
        let [keybinds_area, footer_area] = layout.areas(area);

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
        frame.render_widget(status, keybinds_area);

        self.render_footer(frame, footer_area);
    }

    fn draw(&mut self, frame: &mut Frame) {
        use Constraint::{Length, Min};

        let layout = Layout::vertical([Length(3), Min(0), Length(4)]);
        let [title_area, main_area, status_area] = layout.areas(frame.area());

        self.render_title(frame, title_area);
        self.render_main_area(frame, main_area);
        self.render_status(frame, status_area);
    }
}
