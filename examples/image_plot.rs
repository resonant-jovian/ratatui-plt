//! ImagePlot example: multi-panel layout showing a confusion matrix (matshow),
//! a 2D Gaussian scalar image, and a sparsity pattern (spy).

use std::io;

use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use ratatui_plt::prelude::*;
use ratatui_plt::widgets::image_plot::{ImageData, Interpolation};

fn parse_theme() -> Theme {
    match std::env::args().nth(1).as_deref() {
        Some("light") => Theme::light(),
        Some("minimal") => Theme::minimal(),
        Some("publication") => Theme::publication(),
        Some("solarized") => Theme::solarized(),
        Some("dark") => Theme::dark(),
        None => Theme::auto(),
        Some(other) => {
            eprintln!(
                "Unknown theme '{other}'. Available: dark, light, minimal, publication, solarized"
            );
            std::process::exit(1);
        }
    }
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    Theme::set_default(parse_theme());
    // Panel 1: Confusion matrix via matshow()
    let confusion = vec![
        vec![50.0, 2.0, 1.0, 0.0],
        vec![3.0, 45.0, 4.0, 1.0],
        vec![0.0, 3.0, 42.0, 5.0],
        vec![1.0, 0.0, 6.0, 48.0],
    ];
    let confusion_plot = matshow(confusion)
        .title("Confusion Matrix")
        .colormap(Viridis);

    // Panel 2: 2D Gaussian as scalar image with bilinear interpolation
    let size = 64;
    let gaussian: Vec<Vec<f64>> = (0..size)
        .map(|r| {
            (0..size)
                .map(|c| {
                    let x = (c as f64 - size as f64 / 2.0) / (size as f64 / 6.0);
                    let y = (r as f64 - size as f64 / 2.0) / (size as f64 / 6.0);
                    (-(x * x + y * y) / 2.0).exp()
                })
                .collect()
        })
        .collect();
    let gaussian_plot = ImagePlot::new(ImageData::Scalar(gaussian))
        .colormap(Inferno)
        .interpolation(Interpolation::Bilinear)
        .title("2D Gaussian (Bilinear)")
        .show_colorbar(true);

    // Panel 3: spy() of a banded matrix
    let band_size = 20;
    let banded: Vec<Vec<f64>> = (0..band_size)
        .map(|r| {
            (0..band_size)
                .map(|c| {
                    let diff = if r >= c { r - c } else { c - r };
                    if diff <= 2 { 1.0 } else { 0.0 }
                })
                .collect()
        })
        .collect();
    let spy_plot = spy(&banded).title("Spy: Banded Matrix");

    let panel = MultiPanel::new(1, 3)
        .width_ratios(vec![1.0, 1.0, 1.0])
        .gap(1)
        .panel(0, 0, move |area: Rect, buf: &mut Buffer| {
            (&confusion_plot).render(area, buf);
        })
        .panel(0, 1, move |area: Rect, buf: &mut Buffer| {
            (&gaussian_plot).render(area, buf);
        })
        .panel(0, 2, move |area: Rect, buf: &mut Buffer| {
            (&spy_plot).render(area, buf);
        });

    if headless_export(|area, buf| (&panel).render(area, buf))? {
        return Ok(());
    }

    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    loop {
        terminal.draw(|frame| {
            frame.render_widget(&panel, square_area(frame.area()));
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
