//! User-facing display settings. Persisted by the server (Phase 4); for now
//! the display reads them from command-line flags.

use serde::{Deserialize, Serialize};

use crate::color::Rgb;
use crate::theme::Theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScrollMode {
    /// Content jumps one LED column at a time, like a real sign.
    #[default]
    Stepped,
    /// LEDs cross-fade between columns for smoother motion.
    Smooth,
}

/// How the widget area is split into slots.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WidgetLayout {
    /// A big slot on the left, a smaller one on the right.
    #[default]
    WideLeft,
    /// A smaller slot on the left, a big one on the right.
    WideRight,
    /// Two equal halves.
    Even,
    /// Three equal columns.
    Three,
    /// One slot across the whole width.
    Single,
}

impl WidgetLayout {
    pub const ALL: [WidgetLayout; 5] = [
        WidgetLayout::WideLeft,
        WidgetLayout::WideRight,
        WidgetLayout::Even,
        WidgetLayout::Three,
        WidgetLayout::Single,
    ];

    /// Relative widths of the slots, left to right.
    pub fn widths(self) -> &'static [f32] {
        match self {
            WidgetLayout::WideLeft => &[0.625, 0.375],
            WidgetLayout::WideRight => &[0.375, 0.625],
            WidgetLayout::Even => &[0.5, 0.5],
            WidgetLayout::Three => &[1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0],
            WidgetLayout::Single => &[1.0],
        }
    }

    pub fn slots(self) -> usize {
        self.widths().len()
    }

    pub fn id(self) -> &'static str {
        match self {
            WidgetLayout::WideLeft => "wide_left",
            WidgetLayout::WideRight => "wide_right",
            WidgetLayout::Even => "even",
            WidgetLayout::Three => "three",
            WidgetLayout::Single => "single",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            WidgetLayout::WideLeft => "Wide left",
            WidgetLayout::WideRight => "Wide right",
            WidgetLayout::Even => "Halves",
            WidgetLayout::Three => "Three",
            WidgetLayout::Single => "Single",
        }
    }

    pub fn from_id(id: &str) -> Option<WidgetLayout> {
        WidgetLayout::ALL.into_iter().find(|l| l.id() == id)
    }
}

/// The resolution the display draws at.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Resolution {
    /// The screen's own, but no more than 1080p (4K TVs get 1080p).
    #[default]
    Auto,
    /// The screen's own, whatever it is.
    Native,
    #[serde(rename = "2160p")]
    P2160,
    #[serde(rename = "1440p")]
    P1440,
    #[serde(rename = "1080p")]
    P1080,
    #[serde(rename = "720p")]
    P720,
}

impl Resolution {
    pub const ALL: [Resolution; 6] = [
        Resolution::Auto,
        Resolution::P2160,
        Resolution::P1440,
        Resolution::P1080,
        Resolution::P720,
        Resolution::Native,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Resolution::Auto => "auto",
            Resolution::Native => "native",
            Resolution::P2160 => "2160p",
            Resolution::P1440 => "1440p",
            Resolution::P1080 => "1080p",
            Resolution::P720 => "720p",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Resolution::Auto => "Automatic (1080p on a 4K TV)",
            Resolution::Native => "The screen's own",
            Resolution::P2160 => "4K (2160p)",
            Resolution::P1440 => "1440p",
            Resolution::P1080 => "1080p",
            Resolution::P720 => "720p",
        }
    }

    pub fn from_id(id: &str) -> Option<Resolution> {
        Self::ALL.into_iter().find(|r| r.id() == id)
    }

    /// The TV output mode to ask the screen manager for: "1920x1080@60",
    /// or "native" for the screen's own. Automatic asks for 1080p.
    pub fn tv_mode(self, max_fps: u16) -> String {
        let size = match self {
            Resolution::Native => return "native".into(),
            Resolution::P2160 => "3840x2160",
            Resolution::P1440 => "2560x1440",
            Resolution::Auto | Resolution::P1080 => "1920x1080",
            Resolution::P720 => "1280x720",
        };
        format!("{size}@{max_fps}")
    }

    /// The tallest drawing height, or `None` for the screen's own.
    pub fn max_height(self) -> Option<u32> {
        match self {
            Resolution::Auto | Resolution::P1080 => Some(1080),
            Resolution::Native => None,
            Resolution::P2160 => Some(2160),
            Resolution::P1440 => Some(1440),
            Resolution::P720 => Some(720),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DisplayConfig {
    /// Fraction of screen height used by the header bar (LIVE badge, league
    /// filter, clock); 0 hides it.
    pub header_ratio: f32,
    /// Fraction of screen height used by ticker + crawl.
    pub ticker_ratio: f32,
    /// Fraction of the ticker area given to the crawl.
    pub crawl_share: f32,
    /// LED rows in the main ticker. 17+ allows stacked two-line games.
    pub ticker_rows: u32,
    /// Unused since the crawl became flat text; kept so older settings load.
    pub crawl_rows: u32,
    pub led_color: Rgb,
    /// Main ticker speed in LED columns per second.
    pub ticker_speed: f32,
    /// Crawl speed, in tenths of the crawl's height per second (the same pace
    /// as the old 10-row LED crawl at this many columns per second).
    pub crawl_speed: f32,
    pub scroll_mode: ScrollMode,
    /// Glow (bloom) strength, 0 = off.
    pub glow: f32,
    /// Flicker amount, 0 = off.
    pub flicker: f32,
    /// Lit dot diameter as a fraction of the LED pitch.
    pub dot_size: f32,
    /// How the widget area is split.
    pub widget_layout: WidgetLayout,
    /// The resolution to draw at; lower than the screen's is scaled up
    /// (a Pi 4 can't fill 4K smoothly).
    pub resolution: Resolution,
    /// Frames per second at most: 60 for the smoothest scrolling, 30 to
    /// keep a Pi cooler.
    pub max_fps: u16,
    /// How the crawl and widgets look.
    pub theme: Theme,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            header_ratio: 0.067,
            ticker_ratio: 1.0 / 3.0,
            crawl_share: 0.28,
            ticker_rows: 19,
            crawl_rows: 10,
            led_color: Rgb::AMBER,
            ticker_speed: 15.0,
            crawl_speed: 20.0,
            scroll_mode: ScrollMode::Stepped,
            glow: 0.55,
            flicker: 0.25,
            dot_size: 0.78,
            widget_layout: WidgetLayout::default(),
            resolution: Resolution::default(),
            max_fps: 60,
            theme: Theme::default(),
        }
    }
}

impl DisplayConfig {
    /// Clamps every field into its supported range.
    pub fn sanitized(mut self) -> Self {
        let d = Self::default();
        let clamp = |v: f32, lo: f32, hi: f32, default: f32| if v.is_finite() { v.clamp(lo, hi) } else { default };
        self.header_ratio = clamp(self.header_ratio, 0.0, 0.12, d.header_ratio);
        self.ticker_ratio = clamp(self.ticker_ratio, 0.15, 0.6, d.ticker_ratio);
        self.crawl_share = clamp(self.crawl_share, 0.0, 0.5, d.crawl_share);
        self.ticker_rows = self.ticker_rows.clamp(9, 48);
        self.crawl_rows = self.crawl_rows.clamp(9, 24);
        self.ticker_speed = clamp(self.ticker_speed, 1.0, 200.0, d.ticker_speed);
        self.crawl_speed = clamp(self.crawl_speed, 1.0, 200.0, d.crawl_speed);
        self.glow = clamp(self.glow, 0.0, 2.0, d.glow);
        self.flicker = clamp(self.flicker, 0.0, 1.0, d.flicker);
        self.dot_size = clamp(self.dot_size, 0.3, 1.0, d.dot_size);
        self.max_fps = if self.max_fps <= 30 { 30 } else { 60 };
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_already_sane() {
        assert_eq!(DisplayConfig::default().sanitized(), DisplayConfig::default());
    }

    #[test]
    fn resolution_and_frame_rate_settings() {
        for r in Resolution::ALL {
            assert_eq!(Resolution::from_id(r.id()), Some(r));
        }
        assert_eq!(Resolution::default().max_height(), Some(1080), "4K TVs draw at 1080p unless told otherwise");
        assert_eq!(Resolution::Native.max_height(), None);
        let c = DisplayConfig { max_fps: 45, ..Default::default() }.sanitized();
        assert_eq!(c.max_fps, 60, "only 30 or 60");
        assert_eq!(DisplayConfig { max_fps: 0, ..Default::default() }.sanitized().max_fps, 30);
        let old: DisplayConfig = serde_json::from_str("{}").unwrap();
        assert_eq!((old.resolution, old.max_fps), (Resolution::Auto, 60), "saved before these existed");
    }

    #[test]
    fn out_of_range_values_are_clamped() {
        let c =
            DisplayConfig { ticker_ratio: 5.0, ticker_rows: 2, glow: f32::NAN, flicker: -1.0, ..Default::default() }
                .sanitized();
        assert_eq!(c.ticker_ratio, 0.6);
        assert_eq!(c.ticker_rows, 9);
        assert_eq!(c.glow, DisplayConfig::default().glow);
        assert_eq!(c.flicker, 0.0);
    }

    #[test]
    fn partial_json_fills_defaults() {
        let c: DisplayConfig = serde_json::from_str(r##"{"led_color":"#00ff00"}"##).unwrap();
        assert_eq!(c.led_color, Rgb::new(0, 255, 0));
        assert_eq!(c.ticker_rows, DisplayConfig::default().ticker_rows);
    }
}
