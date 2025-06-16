use regcli::app::App;

fn main() -> std::io::Result<()> {
    let mut app = App::new();
    let mut terminal = ratatui::init();

    app.run(&mut terminal)?;

    ratatui::restore();
    Ok(())
}
