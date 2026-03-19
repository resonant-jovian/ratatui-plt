use std::io;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::prelude::*;
use ratatui_sim::prelude::*;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let panel = MultiPanel::new(2, 2)
        .width_ratios(vec![1.0, 1.0])
        .height_ratios(vec![1.0, 1.0])
        .gap(1)
        // Top-left: horizontal gradient using direct buffer writes
        .panel(0, 0, |area: Rect, buf: &mut Buffer| {
            // Title
            let title = "Panel 1: Gradient";
            let start = area.x + area.width.saturating_sub(title.len() as u16) / 2;
            for (i, ch) in title.chars().enumerate() {
                let x = start + i as u16;
                if x < area.x + area.width {
                    buf[(x, area.y)].set_char(ch).set_fg(Color::White);
                }
            }
            // Draw a horizontal color gradient
            for y in (area.y + 1)..area.y + area.height {
                for x in area.x..area.x + area.width {
                    let t = (x - area.x) as f64 / area.width.max(1) as f64;
                    let r = (t * 255.0) as u8;
                    let b = ((1.0 - t) * 255.0) as u8;
                    buf[(x, y)]
                        .set_char('█')
                        .set_fg(Color::Rgb(r, 50, b));
                }
            }
        })
        // Top-right: checkerboard pattern
        .panel(0, 1, |area: Rect, buf: &mut Buffer| {
            let title = "Panel 2: Checkerboard";
            let start = area.x + area.width.saturating_sub(title.len() as u16) / 2;
            for (i, ch) in title.chars().enumerate() {
                let x = start + i as u16;
                if x < area.x + area.width {
                    buf[(x, area.y)].set_char(ch).set_fg(Color::White);
                }
            }
            for y in (area.y + 1)..area.y + area.height {
                for x in area.x..area.x + area.width {
                    let parity = ((x - area.x) + (y - area.y)) % 2;
                    let (ch, color) = if parity == 0 {
                        ('█', Color::DarkGray)
                    } else {
                        ('░', Color::Gray)
                    };
                    buf[(x, y)].set_char(ch).set_fg(color);
                }
            }
        })
        // Bottom-left: vertical bars
        .panel(1, 0, |area: Rect, buf: &mut Buffer| {
            let title = "Panel 3: Bars";
            let start = area.x + area.width.saturating_sub(title.len() as u16) / 2;
            for (i, ch) in title.chars().enumerate() {
                let x = start + i as u16;
                if x < area.x + area.width {
                    buf[(x, area.y)].set_char(ch).set_fg(Color::White);
                }
            }
            let colors = [Color::Red, Color::Green, Color::Blue, Color::Yellow, Color::Cyan];
            let bar_width = area.width / colors.len() as u16;
            for (i, &color) in colors.iter().enumerate() {
                let bx = area.x + i as u16 * bar_width;
                // Bar height proportional to index
                let bar_h = ((i + 1) as u16 * (area.height.saturating_sub(1))) / colors.len() as u16;
                let top = area.y + area.height - bar_h;
                for y in top..area.y + area.height {
                    for x in bx..bx + bar_width {
                        if x < area.x + area.width {
                            buf[(x, y)].set_char('█').set_fg(color);
                        }
                    }
                }
            }
        })
        // Bottom-right: sine wave drawn with characters
        .panel(1, 1, |area: Rect, buf: &mut Buffer| {
            let title = "Panel 4: Sine";
            let start = area.x + area.width.saturating_sub(title.len() as u16) / 2;
            for (i, ch) in title.chars().enumerate() {
                let x = start + i as u16;
                if x < area.x + area.width {
                    buf[(x, area.y)].set_char(ch).set_fg(Color::White);
                }
            }
            let h = area.height.saturating_sub(1) as f64;
            let mid = area.y + 1 + area.height.saturating_sub(2) / 2;
            for x in area.x..area.x + area.width {
                let t = (x - area.x) as f64 / area.width.max(1) as f64 * 4.0 * std::f64::consts::PI;
                let sy = mid as f64 - t.sin() * (h / 2.0 - 1.0);
                let yi = sy.round() as u16;
                if yi > area.y && yi < area.y + area.height {
                    buf[(x, yi)].set_char('●').set_fg(Color::Cyan);
                }
            }
        });

    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            frame.render_widget(&panel, area);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press
                && (key.code == KeyCode::Char('q') || key.code == KeyCode::Esc)
            {
                break;
            }
        }
    }

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
