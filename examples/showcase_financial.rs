//! Showcase: Financial visualization widgets.
//!
//! 6 plots in a 2x3 mosaic grid (ABC / DEF):
//! A) CandlestickChart, B) WaterfallChart, C) GaugeChart,
//! D) GanttChart, E) FunnelChart, F) FunnelArea.

use std::io;

use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use ratatui_plt::prelude::*;
use ratatui_plt::widgets::candlestick::{Candle, CandlestickChart};
use ratatui_plt::widgets::funnel::{FunnelChart, FunnelEntry};
use ratatui_plt::widgets::funnel_area::{FunnelArea, FunnelAreaEntry};
use ratatui_plt::widgets::gantt::{GanttChart, GanttTask};
use ratatui_plt::widgets::gauge::{GaugeChart, GaugeSector};
use ratatui_plt::widgets::waterfall::{WaterfallChart, WaterfallEntry};

/// Simple deterministic LCG pseudo-random number generator.
fn lcg(seed: &mut u64) -> f64 {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
    (*seed >> 33) as f64 / (1u64 << 31) as f64
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    Theme::set_default(Theme::light());
    let theme = Theme::get_default();
    let c0 = theme.color_cycle.at(0);
    let c1 = theme.color_cycle.at(1);
    let c2 = theme.color_cycle.at(2);
    let c3 = theme.color_cycle.at(3);
    let c4 = theme.color_cycle.at(4);

    // ---- Panel A: CandlestickChart — 30 days OHLC ----
    let mut seed = 42u64;
    let mut price = 100.0;
    let candles: Vec<Candle> = (1..=30)
        .map(|day| {
            let open = price;
            let change = (lcg(&mut seed) - 0.48) * 6.0;
            let close = open + change;
            let high = open.max(close) + lcg(&mut seed) * 3.0;
            let low = open.min(close) - lcg(&mut seed) * 3.0;
            price = close;
            Candle::new(day as f64, open, high, low, close)
        })
        .collect();

    let candlestick = CandlestickChart::new()
        .candles(candles)
        .title("30-Day OHLC")
        .x_axis(Axis::new().label("Day").grid(true))
        .y_axis(Axis::new().label("Price").grid(true));

    // ---- Panel B: WaterfallChart — P&L breakdown ----
    let waterfall = WaterfallChart::new()
        .entry(WaterfallEntry::new("Revenue", 500.0))
        .entry(WaterfallEntry::new("COGS", -180.0))
        .entry(WaterfallEntry::total("Gross", 320.0))
        .entry(WaterfallEntry::new("SGA", -120.0))
        .entry(WaterfallEntry::new("R&D", -60.0))
        .entry(WaterfallEntry::new("Tax", -35.0))
        .entry(WaterfallEntry::total("Net", 105.0))
        .title("P&L Waterfall")
        .y_axis(Axis::new().label("$ (thousands)").grid(true));

    // ---- Panel C: GaugeChart — Customer Satisfaction KPI ----
    let gauge = GaugeChart::new(78.0)
        .min(0.0)
        .max(100.0)
        .sector(GaugeSector::new(0.0, 40.0, theme.negative_color))
        .sector(GaugeSector::new(40.0, 70.0, theme.neutral_color))
        .sector(GaugeSector::new(70.0, 100.0, theme.positive_color))
        .label("78% CSAT")
        .title("Customer Satisfaction");

    // ---- Panel D: GanttChart — 5-task project timeline ----
    let gantt = GanttChart::new()
        .task(GanttTask::new("Research").segment(0.0, 3.0).color(c0))
        .task(GanttTask::new("Design").segment(2.0, 4.0).color(c1))
        .task(GanttTask::new("Develop").segment(5.0, 6.0).color(c2))
        .task(GanttTask::new("Test").segment(9.0, 3.0).color(c3))
        .task(GanttTask::new("Deploy").segment(11.0, 2.0).color(c4))
        .x_axis(Axis::new().label("Week").grid(true))
        .show_grid(true)
        .title("Project Timeline");

    // ---- Panel E: FunnelChart — sales conversion pipeline ----
    let funnel = FunnelChart::new()
        .entry(FunnelEntry::new("Visitors", 10000.0).color(c0))
        .entry(FunnelEntry::new("Leads", 5200.0).color(c1))
        .entry(FunnelEntry::new("Qualified", 2800.0).color(c2))
        .entry(FunnelEntry::new("Proposals", 1400.0).color(c3))
        .entry(FunnelEntry::new("Won", 600.0).color(c4))
        .title("Sales Funnel");

    // ---- Panel F: FunnelArea — same pipeline, trapezoidal ----
    let funnel_area = FunnelArea::new()
        .entry(FunnelAreaEntry::new("Visitors", 10000.0).color(c0))
        .entry(FunnelAreaEntry::new("Leads", 5200.0).color(c1))
        .entry(FunnelAreaEntry::new("Qualified", 2800.0).color(c2))
        .entry(FunnelAreaEntry::new("Proposals", 1400.0).color(c3))
        .entry(FunnelAreaEntry::new("Won", 600.0).color(c4))
        .title("Sales Funnel (Area)");

    // ---- Assemble 2x3 mosaic: ABC / DEF ----
    let panel = MultiPanel::from_mosaic("ABC\nDEF")
        .gap(1)
        .suptitle("Financial Plots Showcase (q to quit)")
        .mosaic_panel('A', move |area: Rect, buf: &mut Buffer| {
            (&candlestick).render(area, buf);
        })
        .mosaic_panel('B', move |area: Rect, buf: &mut Buffer| {
            (&waterfall).render(area, buf);
        })
        .mosaic_panel('C', move |area: Rect, buf: &mut Buffer| {
            (&gauge).render(area, buf);
        })
        .mosaic_panel('D', move |area: Rect, buf: &mut Buffer| {
            (&gantt).render(area, buf);
        })
        .mosaic_panel('E', move |area: Rect, buf: &mut Buffer| {
            (&funnel).render(area, buf);
        })
        .mosaic_panel('F', move |area: Rect, buf: &mut Buffer| {
            (&funnel_area).render(area, buf);
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
