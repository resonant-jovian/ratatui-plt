//! Theme & config example: ThemeGuard and PlotConfig context managers.
//!
//! Shows all five built-in themes side by side in a grid, demonstrating
//! how Theme::activate() scopes theme changes via RAII guards.

use std::io;

use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use ratatui_plt::prelude::*;

fn make_plot(theme: &Theme, title: &str) -> LinePlot {
    let n = 100;
    let mut cycle = theme.color_cycle.clone();
    let c1 = cycle.next_color();
    let c2 = cycle.next_color();

    let s1 = Series::new("sin")
        .data(
            (0..n)
                .map(|i| {
                    let x = i as f64 * 0.06;
                    (x, x.sin())
                })
                .collect(),
        )
        .color(c1);

    let s2 = Series::new("cos")
        .data(
            (0..n)
                .map(|i| {
                    let x = i as f64 * 0.06;
                    (x, x.cos())
                })
                .collect(),
        )
        .color(c2);

    LinePlot::new()
        .series(s1)
        .series(s2)
        .title(title)
        .x_axis(Axis::new().grid(true).minor_grid(true).minor_tick_count(3))
        .y_axis(Axis::new().grid(true))
        .show_legend(true)
        .theme(theme.clone())
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let themes = [
        (Theme::dark(), "Dark"),
        (Theme::light(), "Light"),
        (Theme::minimal(), "Minimal"),
        (Theme::publication(), "Publication"),
        (Theme::solarized(), "Solarized"),
    ];

    let plots: Vec<LinePlot> = themes
        .iter()
        .map(|(t, name)| make_plot(t, &format!("{name} (minor grid)")))
        .collect();

    loop {
        terminal.draw(|frame| {
            let area = frame.area();

            // 2 rows: top has 3, bottom has 2
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(area);

            let top = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Ratio(1, 3),
                    Constraint::Ratio(1, 3),
                    Constraint::Ratio(1, 3),
                ])
                .split(rows[0]);

            let bot = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
                .split(rows[1]);

            frame.render_widget(&plots[0], top[0]);
            frame.render_widget(&plots[1], top[1]);
            frame.render_widget(&plots[2], top[2]);
            frame.render_widget(&plots[3], bot[0]);
            frame.render_widget(&plots[4], bot[1]);
        })?;

        if let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
            && (key.code == KeyCode::Char('q') || key.code == KeyCode::Esc)
        {
            break;
        }
    }

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
