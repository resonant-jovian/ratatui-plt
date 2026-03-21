//! Theme demonstration: all five built-in themes rendered side by side.
//!
//! Each panel shows the same sin/cos data with a different theme applied,
//! illustrating colors, grid styles, and axis appearance.
//!
//! Tip: try switching your terminal between a light and dark background
//! to see how each theme adapts. The "Light" and "Publication" themes
//! work best on light terminal backgrounds; "Dark" and "Solarized" on dark.

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
        .map(|(t, name)| make_plot(t, &format!("Theme: {name}")))
        .collect();

    loop {
        terminal.draw(|frame| {
            let area = frame.area();

            // Header + 2 rows of plots
            let outer = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Percentage(50),
                    Constraint::Percentage(50),
                ])
                .split(area);

            // Render header line
            let header = ratatui::widgets::Paragraph::new(
                "Built-in Theme Gallery  |  q to quit  |  Tip: try light/dark terminal backgrounds",
            )
            .style(
                ratatui::style::Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            )
            .alignment(ratatui::layout::Alignment::Center);
            frame.render_widget(header, outer[0]);

            let top = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Ratio(1, 3),
                    Constraint::Ratio(1, 3),
                    Constraint::Ratio(1, 3),
                ])
                .split(outer[1]);

            let bot = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
                .split(outer[2]);

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
