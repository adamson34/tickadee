//! Turns the admin form (`application/x-www-form-urlencoded` pairs) into
//! settings. Pure, so every field is testable without HTTP.

use chrono::NaiveTime;
use marqueet_core::Rgb;
use marqueet_core::config::{ScrollMode, WidgetLayout};
use marqueet_core::settings::{QuietHours, Settings, TakeoverPolicy, WidgetKind, WidgetSlot};
use marqueet_core::sports::{GameId, LeagueId, TeamId};
use marqueet_core::theme::{Palette, Style, Theme};
use marqueet_core::weather::{Place, Units};

fn field<'a>(pairs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
}

fn all<'a>(pairs: &'a [(String, String)], key: &'a str) -> impl Iterator<Item = &'a str> + 'a {
    pairs.iter().filter(move |(k, _)| k == key).map(|(_, v)| v.as_str())
}

fn number<T: std::str::FromStr>(pairs: &[(String, String)], key: &str, current: T) -> Result<T, String> {
    match field(pairs, key).map(str::trim) {
        None | Some("") => Ok(current),
        Some(v) => v.parse().map_err(|_| format!("{key}: {v:?} is not a number")),
    }
}

/// Slot `i`'s widget and its option (`widget_<i>_<kind>`), if sent.
fn slot(pairs: &[(String, String)], i: usize) -> Option<Result<WidgetSlot, String>> {
    let v = field(pairs, &format!("widget_{i}"))?;
    Some(WidgetKind::from_id(v).ok_or_else(|| format!("unknown widget {v:?}")).map(|kind| WidgetSlot {
        kind,
        option:
            field(pairs, &format!("widget_{i}_{}", kind.id())).map(str::trim).filter(|o| !o.is_empty()).map(Into::into),
    }))
}

fn time(v: &str) -> Result<NaiveTime, String> {
    NaiveTime::parse_from_str(v.trim(), "%H:%M").map_err(|_| format!("{v:?} is not a time (HH:MM)"))
}

/// What to do with the weather location field.
#[derive(Clone, Debug, PartialEq)]
pub enum LocationChange {
    Keep,
    Clear,
    Set(Place),
    /// A place name to look up.
    Lookup(String),
}

/// Reads the `location` field: blank clears it, the current place's name
/// keeps it, "lat, lon" sets it directly, anything else is looked up.
pub fn location(pairs: &[(String, String)], current: Option<&Place>) -> LocationChange {
    let Some(v) = field(pairs, "location").map(str::trim) else { return LocationChange::Keep };
    if v.is_empty() {
        LocationChange::Clear
    } else if current.is_some_and(|p| p.name == v) {
        LocationChange::Keep
    } else if let Some(p) = Place::from_coordinates(v) {
        LocationChange::Set(p)
    } else {
        LocationChange::Lookup(v.to_owned())
    }
}

/// The theme after the Look section: a pasted code wins; else a different
/// look (or "go back") starts from that look's colors; else the submitted
/// colors apply to the current look.
fn theme(current: &Theme, style: &str, pairs: &[(String, String)]) -> Result<Theme, String> {
    if let Some(code) = field(pairs, "theme_code").filter(|c| !c.trim().is_empty()) {
        return Theme::from_code(code);
    }
    let style = Style::from_id(style).ok_or_else(|| format!("unknown look {style:?}"))?;
    let mut theme = *current;
    if style != current.style || field(pairs, "theme_reset") == Some("on") {
        theme = Theme::preset(style);
    } else {
        for (role, label) in Palette::ROLES {
            if let Some(v) = field(pairs, &format!("color_{role}")) {
                let color = v.parse::<Rgb>().map_err(|_| format!("{label}: {v:?} isn't a color"))?;
                theme.palette.set(role, color);
            }
        }
    }
    theme.team_colors = field(pairs, "team_colors") == Some("on");
    Ok(theme)
}

/// Settings after applying the submitted form to `current`. Leagues are the
/// checked `league` values ordered by their `order_<id>` fields; unknown
/// fields are ignored. The result is sanitized.
pub fn apply(current: &Settings, supported: &[LeagueId], pairs: &[(String, String)]) -> Result<Settings, String> {
    let mut s = current.clone();

    let mut leagues: Vec<(i64, usize, LeagueId)> = Vec::new();
    for id in all(pairs, "league") {
        let league = LeagueId::new(id);
        let Some(pos) = supported.iter().position(|l| *l == league) else {
            return Err(format!("unknown league {id:?}"));
        };
        let order = number(pairs, &format!("order_{id}"), i64::MAX)?;
        leagues.push((order, pos, league));
    }
    if leagues.is_empty() {
        return Err("pick at least one league".into());
    }
    leagues.sort();
    s.leagues = leagues.into_iter().map(|(_, _, l)| l).collect();

    s.favorites = all(pairs, "favorite").filter(|v| !v.is_empty()).map(|v| TeamId(v.to_owned())).collect();

    if let Some(v) = field(pairs, "takeovers") {
        s.takeovers = match v {
            "all" => TakeoverPolicy::All,
            "favorites" => TakeoverPolicy::Favorites,
            "off" => TakeoverPolicy::Off,
            other => return Err(format!("unknown takeover setting {other:?}")),
        };
    }

    if let Some(v) = field(pairs, "widget_layout") {
        s.display.widget_layout = WidgetLayout::from_id(v).ok_or_else(|| format!("unknown layout {v:?}"))?;
    }
    // One widget per slot of the layout; the form has a select for every
    // possible slot and extra ones are ignored.
    let slots: Vec<_> = (0..s.display.widget_layout.slots()).map(|i| slot(pairs, i)).collect();
    if slots.iter().all(Option::is_some) {
        s.widgets = slots.into_iter().flatten().collect::<Result<_, _>>()?;
    }

    if let Some(v) = field(pairs, "theme_style") {
        s.display.theme = theme(&s.display.theme, v, pairs)?;
    }

    let d = &mut s.display;
    if let Some(v) = field(pairs, "led_color") {
        d.led_color = v.parse::<Rgb>().map_err(|e| e.to_string())?;
    }
    if let Some(v) = field(pairs, "scroll_mode") {
        d.scroll_mode = match v {
            "stepped" => ScrollMode::Stepped,
            "smooth" => ScrollMode::Smooth,
            other => return Err(format!("unknown scroll mode {other:?}")),
        };
    }
    d.ticker_speed = number(pairs, "ticker_speed", d.ticker_speed)?;
    d.crawl_speed = number(pairs, "crawl_speed", d.crawl_speed)?;
    d.ticker_rows = number(pairs, "ticker_rows", d.ticker_rows)?;
    d.ticker_ratio = number(pairs, "ticker_ratio", d.ticker_ratio)?;
    if let Some(v) = field(pairs, "resolution") {
        d.resolution =
            marqueet_core::config::Resolution::from_id(v).ok_or_else(|| format!("unknown resolution {v:?}"))?;
    }
    d.max_fps = number(pairs, "max_fps", d.max_fps)?;
    d.glow = number(pairs, "glow", d.glow)?;
    d.flicker = number(pairs, "flicker", d.flicker)?;

    s.provider_logos = field(pairs, "provider_logos") == Some("on");
    s.show_odds = field(pairs, "show_odds") == Some("on");
    s.spotlight.auto = field(pairs, "spotlight_auto") == Some("on");
    s.spotlight.favorites = field(pairs, "spotlight_favorites") == Some("on");
    s.spotlight.primetime = field(pairs, "spotlight_primetime") == Some("on");
    s.spotlight.tracker = field(pairs, "spotlight_tracker") == Some("on");
    s.spotlight.game = field(pairs, "spotlight_game").filter(|g| !g.is_empty()).map(|g| GameId(g.to_owned()));
    s.weather.ticker = field(pairs, "weather_ticker") == Some("on");
    s.weather.alerts = field(pairs, "weather_alerts") == Some("on");
    if let Some(v) = field(pairs, "units") {
        s.weather.units = match v {
            "fahrenheit" => Units::Fahrenheit,
            "celsius" => Units::Celsius,
            other => return Err(format!("unknown units {other:?}")),
        };
    }

    if let Some(v) = field(pairs, "time_zone") {
        s.time_zone = Some(v.to_owned());
    }

    s.quiet_hours = match field(pairs, "quiet_enabled") {
        Some("on") => {
            let from = time(field(pairs, "quiet_from").unwrap_or("23:00"))?;
            let to = time(field(pairs, "quiet_to").unwrap_or("07:00"))?;
            Some(QuietHours { from, to, dim: field(pairs, "quiet_look") == Some("dim") })
        }
        _ => None,
    };
    Ok(s.sanitized())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pairs(v: &[(&str, &str)]) -> Vec<(String, String)> {
        v.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect()
    }

    fn supported() -> Vec<LeagueId> {
        ["nfl", "mlb", "nhl", "epl"].map(LeagueId::new).to_vec()
    }

    #[test]
    fn full_form() {
        let form = pairs(&[
            ("league", "nfl"),
            ("order_nfl", "2"),
            ("league", "epl"),
            ("order_epl", "1"),
            ("order_mlb", "0"), // not checked: ignored
            ("favorite", "espn:nfl:2"),
            ("favorite", "espn:epl:364"),
            ("takeovers", "favorites"),
            ("widget_0", "scores"),
            ("widget_1", "game_of_the_day"),
            ("led_color", "#33ccff"),
            ("ticker_speed", "30"),
            ("ticker_rows", "21"),
            ("ticker_ratio", "0.45"),
            ("show_odds", "on"),
            ("resolution", "720p"),
            ("max_fps", "30"),
            ("glow", "0.4"),
            ("scroll_mode", "smooth"),
            ("quiet_enabled", "on"),
            ("quiet_from", "23:30"),
            ("quiet_to", "06:45"),
            ("time_zone", " America/Denver "),
        ]);
        let s = apply(&Settings::default(), &supported(), &form).unwrap();
        assert_eq!(s.leagues, vec![LeagueId::new("epl"), LeagueId::new("nfl")], "ordered by order_<id>");
        assert_eq!(s.favorites.len(), 2);
        assert_eq!(s.takeovers, TakeoverPolicy::Favorites);
        let kinds: Vec<WidgetKind> = s.widgets.iter().map(|w| w.kind).collect();
        assert_eq!(kinds, vec![WidgetKind::Scores, WidgetKind::GameOfTheDay]);
        assert_eq!(s.display.led_color, Rgb::new(0x33, 0xcc, 0xff));
        assert_eq!((s.display.ticker_speed, s.display.ticker_rows, s.display.glow), (30.0, 21, 0.4));
        assert!((s.display.ticker_ratio - 0.45).abs() < 1e-6, "ticker size");
        assert!(s.show_odds);
        assert_eq!((s.display.resolution, s.display.max_fps), (marqueet_core::config::Resolution::P720, 30));
        assert_eq!(s.display.scroll_mode, ScrollMode::Smooth);
        assert_eq!(s.time_zone.as_deref(), Some("America/Denver"));
        let q = s.quiet_hours.unwrap();
        assert_eq!((q.from.to_string(), q.to.to_string()), ("23:30:00".into(), "06:45:00".into()));
    }

    #[test]
    fn layouts_take_one_widget_per_slot() {
        let form = pairs(&[
            ("league", "nfl"),
            ("widget_layout", "three"),
            ("widget_0", "weather"),
            ("widget_1", "scores"),
            ("widget_2", "standings"),
        ]);
        let s = apply(&Settings::default(), &supported(), &form).unwrap();
        assert_eq!(s.display.widget_layout, WidgetLayout::Three);
        let kinds: Vec<WidgetKind> = s.widgets.iter().map(|w| w.kind).collect();
        assert_eq!(kinds, vec![WidgetKind::Weather, WidgetKind::Scores, WidgetKind::Standings]);
        let single =
            pairs(&[("league", "nfl"), ("widget_layout", "single"), ("widget_0", "weather"), ("widget_1", "scores")]);
        assert_eq!(
            apply(&s, &supported(), &single).unwrap().widgets,
            vec![WidgetSlot::from(WidgetKind::Weather)],
            "extra selects ignored"
        );
        let bad = pairs(&[("league", "nfl"), ("widget_layout", "hexagon")]);
        assert!(apply(&s, &supported(), &bad).unwrap_err().contains("layout"));
    }

    #[test]
    fn slot_options_come_from_the_kind_specific_field() {
        let form = pairs(&[
            ("league", "nfl"),
            ("league", "epl"),
            ("widget_layout", "even"),
            ("widget_0", "standings"),
            ("widget_0_standings", "epl"),
            ("widget_0_scores", "nfl"), // another kind's option: ignored
            ("widget_1", "scores"),
            ("widget_1_scores", ""),
        ]);
        let s = apply(&Settings::default(), &supported(), &form).unwrap();
        assert_eq!(s.widgets[0].option.as_deref(), Some("epl"));
        assert_eq!(s.widgets[1].option, None, "blank means all leagues");
        let stale = pairs(&[
            ("league", "nfl"),
            ("widget_layout", "single"),
            ("widget_0", "scores"),
            ("widget_0_scores", "epl"),
        ]);
        assert_eq!(apply(&s, &supported(), &stale).unwrap().widgets[0].option, None, "not a followed league");
    }

    #[test]
    fn location_field() {
        let kc = Place { name: "Kansas City, Missouri".into(), latitude: 39.1, longitude: -94.58, time_zone: None };
        let loc = |v: &str| location(&pairs(&[("location", v)]), Some(&kc));
        assert_eq!(location(&[], Some(&kc)), LocationChange::Keep, "field absent");
        assert_eq!(loc(" Kansas City, Missouri "), LocationChange::Keep, "unchanged");
        assert_eq!(loc(""), LocationChange::Clear);
        assert_eq!(loc("Oslo"), LocationChange::Lookup("Oslo".into()));
        assert!(matches!(loc("59.91, 10.75"), LocationChange::Set(p) if p.latitude == 59.91));
        let s = apply(&Settings::default(), &supported(), &pairs(&[("league", "nfl"), ("units", "celsius")])).unwrap();
        assert_eq!(s.weather.units, Units::Celsius);
        assert!(!s.weather.ticker, "unticked");
        let s = apply(&s, &supported(), &pairs(&[("league", "nfl"), ("weather_ticker", "on")])).unwrap();
        assert!(s.weather.ticker);
    }

    #[test]
    fn unchecked_extras_keep_current_values() {
        let current = Settings { quiet_hours: None, ..Settings::default() };
        let s = apply(&current, &supported(), &pairs(&[("league", "mlb")])).unwrap();
        assert_eq!(s.leagues, vec![LeagueId::new("mlb")]);
        assert_eq!(s.display, current.display);
        assert!(s.favorites.is_empty(), "an unticked favorite is removed");
    }

    fn look(extra: &[(&str, &str)]) -> Vec<(String, String)> {
        let mut v = vec![("league", "nfl")];
        v.extend_from_slice(extra);
        pairs(&v)
    }

    #[test]
    fn picking_a_look_uses_its_own_colors() {
        let s = apply(
            &Settings::default(),
            &supported(),
            &look(&[("theme_style", "ballpark"), ("team_colors", "on"), ("color_accent", "#123456")]),
        )
        .unwrap();
        assert_eq!(s.display.theme, Theme::preset(Style::Ballpark), "a new look ignores the old look's color fields");
        let s = apply(&Settings::default(), &supported(), &look(&[("theme_style", "varsity")])).unwrap();
        assert!(!s.display.theme.team_colors, "unticked");
    }

    #[test]
    fn changing_colors_builds_your_own() {
        let form = look(&[
            ("theme_style", "broadcast"),
            ("team_colors", "on"),
            ("color_accent", "#123456"),
            ("color_ground", "#000000"),
        ]);
        let s = apply(&Settings::default(), &supported(), &form).unwrap();
        let t = s.display.theme;
        assert_eq!(
            (t.style, t.palette.accent, t.palette.ground),
            (Style::Broadcast, Rgb::new(0x12, 0x34, 0x56), Rgb::BLACK)
        );
        assert!(!t.is_preset() && t.team_colors);
        // Going back to the look's own colors.
        let back = apply(
            &s,
            &supported(),
            &look(&[
                ("theme_style", "broadcast"),
                ("team_colors", "on"),
                ("color_accent", "#123456"),
                ("theme_reset", "on"),
            ]),
        )
        .unwrap();
        assert!(back.display.theme.is_preset());
    }

    #[test]
    fn a_pasted_code_wins() {
        let mut shared = Theme::preset(Style::Varsity);
        shared.palette.live = Rgb::new(0, 0xff, 0);
        let code = shared.code();
        let s =
            apply(&Settings::default(), &supported(), &look(&[("theme_style", "broadcast"), ("theme_code", &code)]))
                .unwrap();
        assert_eq!(s.display.theme, shared);
        let bad =
            apply(&Settings::default(), &supported(), &look(&[("theme_style", "broadcast"), ("theme_code", "nope")]));
        assert!(bad.unwrap_err().contains("theme code"));
        let blank =
            apply(&Settings::default(), &supported(), &look(&[("theme_style", "ballpark"), ("theme_code", "  ")]))
                .unwrap();
        assert_eq!(blank.display.theme.style, Style::Ballpark, "an empty code box is ignored");
    }

    #[test]
    fn bad_input_is_explained() {
        let bad = |form: &[(&str, &str)]| apply(&Settings::default(), &supported(), &pairs(form)).unwrap_err();
        assert!(bad(&[]).contains("at least one league"));
        assert!(bad(&[("league", "curling")]).contains("unknown league"));
        assert!(bad(&[("league", "nfl"), ("ticker_speed", "fast")]).contains("not a number"));
        assert!(bad(&[("league", "nfl"), ("led_color", "orange-ish")]).contains("invalid color"));
        assert!(bad(&[("league", "nfl"), ("quiet_enabled", "on"), ("quiet_from", "late")]).contains("not a time"));
        assert!(bad(&[("league", "nfl"), ("takeovers", "sometimes")]).contains("takeover"));
    }

    #[test]
    fn values_are_clamped_by_sanitize() {
        let s =
            apply(&Settings::default(), &supported(), &pairs(&[("league", "nfl"), ("ticker_rows", "500")])).unwrap();
        assert_eq!(s.display.ticker_rows, 48);
        let big =
            apply(&Settings::default(), &supported(), &pairs(&[("league", "nfl"), ("ticker_ratio", "3")])).unwrap();
        assert!((big.display.ticker_ratio - 0.6).abs() < 1e-6, "the ticker can't take the whole screen");
    }
}
