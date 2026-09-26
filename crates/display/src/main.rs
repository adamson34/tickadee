//! Marqueet display: a native full-screen LED ticker.
//!
//! Phase 1 runs on built-in mock data. Examples:
//!
//! ```text
//! marqueet-display                          # 1920x1080 window
//! marqueet-display --size 1366x768 --led-color green
//! marqueet-display --fullscreen
//! marqueet-display --screenshot out.png --at 4.3 --scroll-to mock:nfl:1
//! ```

mod app;
mod band;
mod crawl;
mod feed;
mod gpu;
mod header;
mod mock;
mod render;
mod scene;
mod screenshot;
mod setup;
mod takeover;
mod theme;
mod ui;
mod upscale;
mod watchdog;
mod weather;
mod widgets;

use std::path::PathBuf;

use clap::Parser;
use marqueet_core::Rgb;
use marqueet_core::config::WidgetLayout;
use marqueet_core::config::{DisplayConfig, ScrollMode};
use marqueet_core::settings::{Settings, WidgetKind};
use marqueet_core::sports::HomeAway;
use marqueet_core::theme::{Style, Theme};
use scene::FeedSource;

#[derive(Debug, Parser)]
#[command(name = "marqueet-display", version, about = "Full-screen LED sports ticker")]
struct Cli {
    /// Window size in physical pixels, e.g. 1366x768 (ignored with --fullscreen).
    #[arg(long, default_value = "1920x1080", value_parser = parse_size)]
    size: (u32, u32),

    /// Borderless full screen on the primary monitor.
    #[arg(long)]
    fullscreen: bool,

    /// Fraction of the screen height used by the ticker and crawl.
    #[arg(long)]
    ticker_ratio: Option<f32>,

    /// Fraction of the ticker area used by the crawl (0 hides it).
    #[arg(long)]
    crawl_share: Option<f32>,

    /// LED rows in the main ticker (17+ shows two-line game blocks).
    #[arg(long)]
    ticker_rows: Option<u32>,

    /// Widget area layout: wide_left, wide_right, even, three or single.
    #[arg(long, value_parser = parse_layout)]
    widget_layout: Option<WidgetLayout>,

    /// Look of the crawl and widgets: broadcast, ballpark or varsity.
    #[arg(long, value_parser = parse_theme)]
    theme: Option<Style>,

    /// LED color: amber, red, green, blue, white or #rrggbb.
    #[arg(long)]
    led_color: Option<Rgb>,

    /// Main ticker speed in LED columns per second.
    #[arg(long)]
    speed: Option<f32>,

    /// Crawl speed, in tenths of the crawl's height per second.
    #[arg(long)]
    crawl_speed: Option<f32>,

    /// Cross-fade between LED columns instead of stepping.
    #[arg(long)]
    smooth: bool,

    /// Glow strength (0 = off).
    #[arg(long)]
    glow: Option<f32>,

    /// Flicker amount (0 = off).
    #[arg(long)]
    flicker: Option<f32>,

    /// Lit dot diameter as a fraction of the LED pitch.
    #[arg(long)]
    dot_size: Option<f32>,

    /// Use built-in demo data instead of connecting to marqueet-server.
    #[arg(long)]
    mock: bool,

    /// marqueet-server feed URL.
    #[arg(long, value_name = "URL", default_value = feed::DEFAULT_URL, conflicts_with = "mock")]
    server: String,

    /// With --mock: seed for the demo data.
    #[arg(long, default_value_t = 7)]
    seed: u64,

    /// With --mock: the two widget slots, e.g. `game_of_the_day,standings`
    /// (game_of_the_day, scores, standings, weather, fantasy, bracket).
    #[arg(long, value_delimiter = ',', value_parser = parse_widget, requires = "mock")]
    widgets: Vec<WidgetKind>,

    /// With --mock: spotlight a demo game (one game filling the widget area):
    /// the featured football game, or the one given (e.g. `mock:mlb:1`).
    #[arg(long, requires = "mock", num_args = 0..=1, default_missing_value = "mock:nfl:1", value_name = "GAME_ID")]
    spotlight: Option<String>,

    /// With --mock: LED art (a PNG, a PNG strip or an animated GIF) for the
    /// demo's takeovers, to preview it (try it with --score).
    #[arg(long, requires = "mock", value_name = "FILE")]
    takeover_art: Option<PathBuf>,

    /// With --takeover-art: frames side by side in a PNG strip.
    #[arg(long, default_value_t = 1)]
    art_frames: u32,

    /// With --takeover-art: milliseconds per frame (a GIF has its own).
    #[arg(long)]
    art_ms: Option<u16>,

    /// With --takeover-art: where it goes: above (the words), intro (on its
    /// own first) or behind (dimmed, behind the words).
    #[arg(long, default_value = "above", value_parser = ["above", "intro", "behind"])]
    art_placement: String,

    /// Render one frame to this PNG file instead of opening a window.
    #[arg(long, value_name = "PNG", conflicts_with = "record")]
    screenshot: Option<PathBuf>,

    /// Render a sequence of PNG frames into this directory (for demo videos).
    #[arg(long, value_name = "DIR")]
    record: Option<PathBuf>,

    /// With --record: seconds to record.
    #[arg(long, default_value_t = 6.0)]
    duration: f64,

    /// With --record: frames per second.
    #[arg(long, default_value_t = 30)]
    fps: u32,

    /// Headless: simulated seconds before capturing (the first frame, when recording).
    #[arg(long, default_value_t = 6.0)]
    at: f64,

    /// Headless: when capture starts, jump the ticker so this segment id is at the left.
    #[arg(long, value_name = "SEGMENT_ID")]
    scroll_to: Option<String>,

    /// Headless: flash this ticker segment id (as if it just scored).
    #[arg(long, value_name = "SEGMENT_ID")]
    flash: Option<String>,

    /// With --flash: simulated second the flash starts (default: just before capture).
    #[arg(long)]
    flash_at: Option<f64>,

    /// Headless: score for a mock game, e.g. `mock:nfl:1:home:7`, as if a live
    /// scoring alert arrived (updates the score and flashes it).
    #[arg(long, value_name = "GAME_ID:home|away:POINTS", value_parser = parse_score)]
    score: Option<(String, HomeAway, u16)>,

    /// With --score: simulated second the score happens (default: just before capture).
    #[arg(long)]
    score_at: Option<f64>,

    /// Headless with --server: run in real time for this many seconds first,
    /// so alerts sent meanwhile (e.g. with curl) are captured.
    #[arg(long, default_value_t = 0.0, conflicts_with = "mock")]
    wait: f64,
}

fn parse_score(s: &str) -> Result<(String, HomeAway, u16), String> {
    let mut parts = s.rsplitn(3, ':');
    let (Some(points), Some(side), Some(id)) = (parts.next(), parts.next(), parts.next()) else {
        return Err("expected GAME_ID:home|away:POINTS, e.g. mock:nfl:1:home:7".into());
    };
    let side = match side {
        "home" => HomeAway::Home,
        "away" => HomeAway::Away,
        _ => return Err("side must be home or away".into()),
    };
    let points = points.parse().map_err(|_| "points must be a whole number")?;
    Ok((id.to_owned(), side, points))
}

fn parse_layout(s: &str) -> Result<WidgetLayout, String> {
    WidgetLayout::from_id(s.trim()).ok_or_else(|| "expected wide_left, wide_right, even, three or single".into())
}

fn parse_widget(s: &str) -> Result<WidgetKind, String> {
    match s.trim() {
        "game_of_the_day" | "gotd" => Ok(WidgetKind::GameOfTheDay),
        "scores" => Ok(WidgetKind::Scores),
        "standings" => Ok(WidgetKind::Standings),
        "weather" => Ok(WidgetKind::Weather),
        "fantasy" => Ok(WidgetKind::Fantasy),
        "bracket" => Ok(WidgetKind::Bracket),
        other => {
            Err(format!("unknown widget {other:?} (game_of_the_day, scores, standings, weather, fantasy, bracket)"))
        }
    }
}

fn parse_theme(s: &str) -> Result<Style, String> {
    Style::from_id(s).ok_or_else(|| format!("unknown theme {s:?} (broadcast, ballpark, varsity)"))
}

fn parse_size(s: &str) -> Result<(u32, u32), String> {
    let (w, h) = s.split_once(['x', 'X']).ok_or("expected WIDTHxHEIGHT, e.g. 1366x768")?;
    let w: u32 = w.trim().parse().map_err(|_| "bad width")?;
    let h: u32 = h.trim().parse().map_err(|_| "bad height")?;
    if !(320..=8192).contains(&w) || !(240..=8192).contains(&h) {
        return Err("size out of range".into());
    }
    Ok((w, h))
}

impl Cli {
    fn source(&self) -> FeedSource {
        if self.mock {
            // Same slot rules as saved settings: one widget per layout slot.
            let mut s = Settings::default();
            s.display.widget_layout = self.widget_layout.unwrap_or_default();
            if !self.widgets.is_empty() {
                s.widgets = self.widgets.iter().map(|k| (*k).into()).collect();
            }
            let widgets = s.sanitized().widgets;
            FeedSource::Mock { seed: self.seed, widgets, spotlight: self.spotlight.clone(), art: self.art() }
        } else {
            FeedSource::Live { url: self.server.clone() }
        }
    }

    /// The --takeover-art file as LED art (logged and skipped if it can't be
    /// used).
    fn art(&self) -> Option<marqueet_core::art::TakeoverArt> {
        use marqueet_core::art::{ArtPlacement, FRAME_MS_DEFAULT, TakeoverArt, decode};
        let path = self.takeover_art.as_ref()?;
        let result = std::fs::read(path).map_err(|e| e.to_string()).and_then(|bytes| {
            let (frames, file_ms) = decode(&bytes, self.art_frames)?;
            let placement = ArtPlacement::from_id(&self.art_placement).unwrap_or_default();
            TakeoverArt::new(frames, self.art_ms.or(file_ms).unwrap_or(FRAME_MS_DEFAULT), placement)
        });
        result.map_err(|e| log::error!("--takeover-art {}: {e}", path.display())).ok()
    }

    fn config(&self) -> DisplayConfig {
        let mut c = DisplayConfig::default();
        macro_rules! set {
            ($($field:ident <- $opt:ident),* $(,)?) => {
                $(if let Some(v) = self.$opt { c.$field = v; })*
            };
        }
        set!(
            ticker_ratio <- ticker_ratio,
            crawl_share <- crawl_share,
            ticker_rows <- ticker_rows,
            led_color <- led_color,
            ticker_speed <- speed,
            crawl_speed <- crawl_speed,
            glow <- glow,
            flicker <- flicker,
            dot_size <- dot_size,
            widget_layout <- widget_layout,
        );
        if let Some(style) = self.theme {
            c.theme = Theme::preset(style);
        }
        if self.smooth {
            c.scroll_mode = ScrollMode::Smooth;
        }
        c.sanitized()
    }
}

fn main() -> render::Result<()> {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info,wgpu_core=warn,wgpu_hal=warn,naga=warn"),
    )
    .init();
    let cli = Cli::parse();
    let config = cli.config();
    let output = match (&cli.screenshot, &cli.record) {
        (Some(png), _) => screenshot::Output::Frame(png.clone()),
        (None, Some(dir)) => screenshot::Output::Frames { dir: dir.clone(), duration: cli.duration, fps: cli.fps },
        (None, None) => return app::run(config, cli.size, cli.fullscreen, cli.source()),
    };
    screenshot::run(
        config,
        screenshot::Options {
            output,
            size: cli.size,
            at: cli.at,
            source: cli.source(),
            scroll_to: cli.scroll_to.clone(),
            flash: cli.flash.clone(),
            flash_at: cli.flash_at.unwrap_or(cli.at - 0.1),
            score: cli.score.clone(),
            score_at: cli.score_at.unwrap_or(cli.at - 0.1),
            wait: cli.wait,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_score_specs() {
        assert_eq!(parse_score("mock:nfl:1:home:7"), Ok(("mock:nfl:1".into(), HomeAway::Home, 7)));
        assert_eq!(parse_score("g:away:3"), Ok(("g".into(), HomeAway::Away, 3)));
        assert!(parse_score("mock:nfl:1:left:7").is_err());
        assert!(parse_score("7").is_err());
    }

    #[test]
    fn parses_sizes() {
        assert_eq!(parse_size("1366x768"), Ok((1366, 768)));
        assert_eq!(parse_size("1024X768"), Ok((1024, 768)));
        assert!(parse_size("1366").is_err());
        assert!(parse_size("10x10").is_err());
    }

    #[test]
    fn flags_override_defaults_and_are_sanitized() {
        let cli = Cli::parse_from(["x", "--led-color", "green", "--ticker-rows", "500", "--smooth"]);
        let c = cli.config();
        assert_eq!(c.led_color, Rgb::GREEN);
        assert_eq!(c.ticker_rows, 48);
        assert_eq!(c.scroll_mode, ScrollMode::Smooth);
    }
}
