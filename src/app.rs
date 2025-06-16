use crossterm::event::{self, Event, KeyCode};
use ratatui::{layout::{Constraint, Layout}, prelude::Backend, style::{Style, Stylize}, text::Line, widgets::{Block, Paragraph}, Frame, Terminal};

pub struct App {

}

impl App {
    pub fn new() -> Self {
        Self {}
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
                _ => (),
            }
            _ => (),
        }

        Ok(false)
    }

    fn draw(&self, frame: &mut Frame) {
        use Constraint::{Length, Min};

        let vertical = Layout::vertical([Length(3), Min(0), Length(3)]);
        let [title_area, main_area, status_area] = vertical.areas(frame.area());

        let bold_style = Style::new().bold();

        let title_block = Block::bordered().title("Regedit").style(bold_style);
        let title_content = Paragraph::new("Computer -> Registry").block(title_block);
        frame.render_widget(title_content, title_area);

        frame.render_widget(Block::bordered(), main_area);

        let status = Line::from(vec![
            " <Esc> ".black().on_white().bold(),
            " Quit ".into(),
        ]);

        frame.render_widget(status, status_area);
    }
}
