//! Dendrogram example: Hierarchical Clustering of Primate Species.
//!
//! 8 primate species clustered by approximate evolutionary divergence times
//! (millions of years ago). The tree reflects real phylogenetic relationships:
//! great apes cluster together, Old World monkeys form an outgroup, etc.

use std::io;

use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use ratatui_plt::prelude::*;

fn parse_theme() -> Theme {
    match std::env::args().nth(1).as_deref() {
        Some("light") => Theme::light(),
        Some("minimal") => Theme::minimal(),
        Some("publication") => Theme::publication(),
        Some("solarized") => Theme::solarized(),
        Some("dark") | None => Theme::dark(),
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
    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    // 8 primate species (leaves indexed 0..8):
    //   0: Human        1: Chimpanzee    2: Gorilla      3: Orangutan
    //   4: Gibbon       5: Macaque       6: Baboon       7: Lemur
    let labels: Vec<String> = vec![
        "Human",
        "Chimp",
        "Gorilla",
        "Orangutan",
        "Gibbon",
        "Macaque",
        "Baboon",
        "Lemur",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    // 7 links (n-1 = 7), building up the tree bottom-to-top.
    // Merged cluster indices start at 8 (= n).
    //
    // Link 0: merge Human(0) + Chimp(1) at distance 6.0 Mya → cluster 8
    // Link 1: merge Macaque(5) + Baboon(6) at distance 8.0 Mya → cluster 9
    // Link 2: merge cluster 8 (Human+Chimp) + Gorilla(2) at 9.0 Mya → cluster 10
    // Link 3: merge cluster 10 + Orangutan(3) at 13.0 Mya → cluster 11
    // Link 4: merge cluster 11 + Gibbon(4) at 20.0 Mya → cluster 12
    // Link 5: merge cluster 12 + cluster 9 (Macaque+Baboon) at 30.0 Mya → cluster 13
    // Link 6: merge cluster 13 + Lemur(7) at 75.0 Mya → cluster 14 (root)
    let links = vec![
        DendroLink::new(0, 1, 6.0),   // Human + Chimp
        DendroLink::new(5, 6, 8.0),   // Macaque + Baboon
        DendroLink::new(8, 2, 9.0),   // (Human+Chimp) + Gorilla
        DendroLink::new(10, 3, 13.0), // African apes + Orangutan
        DendroLink::new(11, 4, 20.0), // Great apes + Gibbon
        DendroLink::new(12, 9, 30.0), // Apes + Old World monkeys
        DendroLink::new(13, 7, 75.0), // Simians + Lemur (root)
    ];

    let dendro = Dendrogram::new(links, labels)
        .title("Hierarchical Clustering: Primate Phylogeny (q to quit)")
        .color_threshold(25.0);

    loop {
        terminal.draw(|frame| {
            frame.render_widget(&dendro, frame.area());
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
