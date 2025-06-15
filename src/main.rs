use crossterm::event::{self, Event, KeyCode};
use ratatui::{layout::{Constraint, Layout}, style::{Style, Stylize}, widgets::Block, Frame};

fn main() -> std::io::Result<()> {
    let mut terminal = ratatui::init();

    loop {
        terminal.draw(draw).expect("Failed to draw frame");
        if handle_events()? {
            break;
        }
    }

    ratatui::restore();
    Ok(())
}

fn handle_events() -> std::io::Result<bool> {
    match event::read()? {
        Event::Key(event) if event.is_press() => match event.code {
            KeyCode::Esc => return Ok(true),
            _ => (),
        }
        _ => (),
    }

    Ok(false)
}

fn draw(frame: &mut Frame) {
    use Constraint::{Length, Min};

    let vertical = Layout::vertical([Length(3), Min(0)]);
    let [title_area, main_area] = vertical.areas(frame.area());

    let bold_style = Style::new().bold();

    frame.render_widget(Block::bordered().title("Regedit").style(bold_style), title_area);
    frame.render_widget(Block::bordered(), main_area);
}
