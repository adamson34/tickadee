//! The admin page as plain server-rendered HTML. Pure: everything it shows is
//! passed in, and every interpolated string goes through [`esc`].

use std::fmt::Write as _;

use chrono::{DateTime, FixedOffset, Utc};
use marqueet_core::alert::{Alert, AlertLevel};
use marqueet_core::config::{ScrollMode, WidgetLayout};
use marqueet_core::provider::LeagueInfo;
use marqueet_core::settings::{Settings, TakeoverPolicy, WidgetKind};
use marqueet_core::sports::{GameId, LeagueId, TeamId};
use marqueet_core::team_art::TeamArtMap;
use marqueet_core::theme::{Palette, Style};
use marqueet_core::weather::Units;

use crate::hub::FantasySearch;

/// A team that can be picked as a favorite.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TeamChoice {
    pub id: TeamId,
    pub league: LeagueId,
    pub name: String,
}

/// A followed fantasy team on the admin page.
#[derive(Clone, Debug)]
pub struct FantasyRow {
    pub league_id: String,
    pub roster_id: u32,
    pub league: String,
    pub team: String,
    /// "Week 3: 100.8 to 127.1", or why not.
    pub status: String,
}

/// A feed on the admin page. Its token isn't here: only a hash is kept, and
/// the token is shown once, when it's made ([`Notice::NewToken`]).
#[derive(Clone, Debug)]
pub struct FeedRow {
    pub name: String,
    pub segments: usize,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Fetch health for one followed league.
#[derive(Clone, Debug)]
pub struct LeagueHealth {
    pub id: LeagueId,
    pub games: usize,
    pub stale: bool,
    pub failures: u32,
    pub last_error: Option<String>,
    pub last_success: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Notice {
    None,
    Saved,
    Error(String),
    /// A feed's new token, shown this once.
    NewToken {
        feed: String,
        token: String,
    },
    PasswordChanged,
    /// A test takeover was sent to the screen.
    Tested,
}

#[derive(Debug)]
pub struct View<'a> {
    pub settings: &'a Settings,
    pub leagues: &'a [LeagueInfo],
    pub teams: &'a [TeamChoice],
    pub health: &'a [LeagueHealth],
    pub alerts: &'a [Alert],
    /// Time zone names for the picker.
    pub zones: &'a [String],
    /// Feeds (no tokens).
    pub feeds: &'a [FeedRow],
    /// Followed fantasy teams.
    pub fantasy: &'a [FantasyRow],
    /// A fantasy account's leagues, after "Find leagues".
    pub fantasy_search: Option<&'a FantasySearch>,
    /// Team colors and logos the person added.
    pub team_art: &'a TeamArtMap,
    /// Today's games, for "watch a game": (id, "NFL · KC at BUF · Q3 4:26").
    pub games: &'a [(GameId, String)],
    /// What the TV output helper last applied, when it's installed.
    pub tv_output: Option<&'a crate::tv_output::Status>,
    /// Teams playing today, favorites first, for test takeovers:
    /// (id, "NFL · Buffalo Blizzard").
    pub playing: &'a [(marqueet_core::sports::TeamId, String)],
    /// This server's address as the browser sees it, for the feed example.
    pub host: &'a str,
    pub notice: Notice,
    /// Logged in over the network (shows "Log out").
    pub remote: bool,
    /// The password was set on this page (not by the device's configuration),
    /// so it can be changed here.
    pub can_change_password: bool,
    pub tz: FixedOffset,
    pub now: DateTime<Utc>,
}

/// The team form's own takeover words: a headline and a second line per
/// play. Blank keeps what's saved.
fn takeover_words_fields(h: &mut String) {
    use marqueet_core::team_art::{WORD_PLAYS, WORDS_HEADLINE_MAX, WORDS_LINE_MAX};
    h.push_str(
        "<details class=\"words\"><summary>Takeover words and LED art</summary><p class=\"hint\">Your own words when this \
         team scores, in place of the usual one (\"KINGDOM TD!\" instead of \"TOUCHDOWN\"), and an optional second \
         line shown under it. Leave a play blank to keep what's there. Try it with <a href=\"#test-takeover\">Test \
         a takeover</a>.</p><div class=\"grid\">",
    );
    for (play, label, usual) in WORD_PLAYS {
        let _ = write!(
            h,
            "<label>{label} <input name=\"words_{play}\" maxlength=\"{WORDS_HEADLINE_MAX}\" placeholder=\"{usual}\" \
             autocomplete=\"off\"></label><label>Second line <input name=\"words_{play}_line\" \
             maxlength=\"{WORDS_LINE_MAX}\" autocomplete=\"off\"></label>"
        );
    }
    h.push_str("</div><label><input type=\"checkbox\" name=\"remove_words\"> Remove this team's words</label>");
    let _ = write!(
        h,
        "<h4>LED art</h4><p class=\"hint\">Your own picture or animation in lights, for this team's takeovers. \
         One light per pixel, so draw small: at most {w}x{hgt} (pixel-art tools like Piskel or Aseprite are \
         ideal). A PNG for a still, an animated GIF, or a PNG strip with the frames side by side.</p>\
         <div class=\"grid\"><label class=\"wide\">Art <input type=\"file\" name=\"art\" \
         accept=\"image/png,image/gif\"></label>\
         <label>Frames in a PNG strip <input type=\"number\" name=\"art_frames\" value=\"1\" min=\"1\" \
         max=\"{frames}\"></label>\
         <label>Speed (ms per frame; a GIF has its own) <input type=\"number\" name=\"art_ms\" min=\"{min}\" \
         max=\"{max}\" placeholder=\"{default}\"></label>\
         <label>Where <select name=\"art_placement\"><option value=\"above\">Above the words</option>\
         <option value=\"intro\">On its own first, then the words</option>\
         <option value=\"behind\">In the background, behind the words</option></select></label></div>\
         <label><input type=\"checkbox\" name=\"remove_art\"> Remove this team's art</label></details>",
        w = marqueet_core::art::ART_MAX_W,
        hgt = marqueet_core::art::ART_MAX_H,
        frames = marqueet_core::art::ART_MAX_FRAMES,
        min = marqueet_core::art::FRAME_MS_MIN,
        max = marqueet_core::art::FRAME_MS_MAX,
        default = marqueet_core::art::FRAME_MS_DEFAULT,
    );
}

/// Buttons that play a takeover on the screen now, to see how it looks.
fn test_takeovers_section(h: &mut String, v: &View) {
    h.push_str(
        "<section id=\"test-takeover\"><h2>Test a takeover</h2><p class=\"hint\">See what a big play or a \
         warning looks like on your screen. Each one replays the last real one of its kind, or makes one up \
         from today's games; pick a team to see it score.</p>\
         <form method=\"post\" action=\"/admin/takeover/test\"><label class=\"wide\">Team \
         <select name=\"team\" aria-label=\"Team\"><option value=\"\">Automatic</option>",
    );
    for (id, label) in v.playing {
        let _ = write!(h, "<option value=\"{}\">{}</option>", esc(&id.0), esc(label));
    }
    h.push_str("</select></label><div class=\"row\">");
    for kind in marqueet_core::test_alerts::TestKind::ALL {
        let _ = write!(h, "<button name=\"kind\" value=\"{}\">{}</button>", kind.id(), kind.label());
    }
    h.push_str("</div></form></section>");
}

/// Escapes text for HTML element content and quoted attribute values.
pub fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

pub(super) fn checked(on: bool) -> &'static str {
    if on { " checked" } else { "" }
}

fn selected(on: bool) -> &'static str {
    if on { " selected" } else { "" }
}

pub(super) fn head(title: &str) -> String {
    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
         <title>{}</title><link rel=\"stylesheet\" href=\"/admin/admin.css\">\
         <script src=\"/admin/admin.js\" defer></script></head><body>",
        esc(title)
    )
}

const BRAND: &str = "<header class=\"top\"><span class=\"brand\">MARQUEET</span><span class=\"sub\">admin</span>";

fn ago(t: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let s = (now - t).num_seconds().max(0);
    match s {
        0..60 => format!("{s}s ago"),
        60..3600 => format!("{}m ago", s / 60),
        _ => format!("{}h ago", s / 3600),
    }
}

/// A league's teams for the picker: (league name, [(team, team name)]).
pub(super) type TeamGroups = Vec<(String, Vec<(TeamId, String)>)>;

/// The team picker (admin page and welcome steps): the picks as tags on top,
/// one search across every league, and each league folded with a count. The
/// checkboxes named `favorite` are what's submitted; admin.js keeps the tags
/// and counts current and runs the search, and without scripting the tags
/// are labels that toggle their checkbox.
pub(super) fn team_picker(groups: &TeamGroups, favorites: &[TeamId]) -> String {
    let mut h = String::from("<div class=\"picker\"><div class=\"picked\" aria-live=\"polite\">");
    let mut n = 0;
    let mut ids = std::collections::HashMap::new();
    for (league, teams) in groups {
        for (id, _) in teams {
            ids.entry(id.clone()).or_insert_with(|| {
                n += 1;
                (format!("fav-{n}"), league.clone())
            });
        }
    }
    let picked: Vec<(&TeamId, &String, &String, &String)> = groups
        .iter()
        .flat_map(|(_, teams)| teams)
        .filter(|(id, _)| favorites.contains(id))
        .filter_map(|(id, name)| ids.get(id).map(|(dom, league)| (id, name, dom, league)))
        .collect();
    if picked.is_empty() {
        h.push_str("<p class=\"picked-empty\">No teams picked yet.</p>");
    }
    let mut shown = std::collections::HashSet::new();
    for (id, name, dom, league) in picked {
        if shown.insert(id) {
            let _ = write!(
                h,
                "<label class=\"chip\" for=\"{dom}\">{} <small>{}</small> <span aria-hidden=\"true\">✕</span></label>",
                esc(name),
                esc(league)
            );
        }
    }
    h.push_str(
        "</div><input type=\"search\" class=\"team-search js-only\" placeholder=\"Search all teams, e.g. Buffalo\" \
         aria-label=\"Search all teams\" autocomplete=\"off\">\
         <p class=\"search-empty js-only\" hidden>No team matches that.</p>",
    );
    let mut first = std::collections::HashSet::new();
    for (league, teams) in groups {
        let picked = teams.iter().filter(|(id, _)| favorites.contains(id)).count();
        let total = format!("{} teams", teams.len());
        let _ = write!(
            h,
            "<details class=\"teams\"><summary>{}<span class=\"count\" data-total=\"{total}\">{}</span></summary><div>",
            esc(league),
            if picked > 0 { format!("{picked} picked") } else { total.clone() },
        );
        if teams.is_empty() {
            h.push_str("<p class=\"empty\">Still loading these teams: refresh this page in a minute.</p>");
        }
        for (id, name) in teams {
            // A team listed in two groups gets one checkbox (the first).
            if !first.insert(id) {
                continue;
            }
            let dom = ids.get(id).map_or("", |(d, _)| d.as_str());
            let _ = write!(
                h,
                "<label class=\"team\"><input type=\"checkbox\" name=\"favorite\" id=\"{dom}\" value=\"{}\" \
                 data-league=\"{}\"{}> {}</label>",
                esc(&id.0),
                esc(league),
                checked(favorites.contains(id)),
                esc(name)
            );
        }
        h.push_str("</div></details>");
    }
    h.push_str("</div>");
    h
}

fn league_name<'a>(leagues: &'a [LeagueInfo], id: &'a LeagueId) -> &'a str {
    leagues.iter().find(|l| &l.id == id).map_or(id.as_str(), |l| l.name.as_str())
}

pub fn render(v: &View<'_>) -> String {
    let s = v.settings;
    let mut h = head("Marqueet admin");
    h.push_str(BRAND);
    if v.remote {
        h.push_str("<form method=\"post\" action=\"/logout\" class=\"logout\"><button>Log out</button></form>");
    }
    h.push_str("</header><main>");
    match &v.notice {
        Notice::None => {}
        Notice::Saved => {
            h.push_str("<p class=\"notice ok\" role=\"status\">Saved. The display updates in a moment.</p>")
        }
        Notice::Error(e) => {
            let _ = write!(h, "<p class=\"notice err\" role=\"alert\">Not saved: {}</p>", esc(e));
        }
        Notice::NewToken { feed, token } => {
            let _ = write!(
                h,
                "<p class=\"notice ok\" role=\"status\">The token for <b>{}</b> is <code class=\"token\">{}</code>. \
                 Copy it now: it isn't shown again (make a new one if you lose it).</p>",
                esc(feed),
                esc(token),
            );
        }
        Notice::Tested => {
            h.push_str("<p class=\"notice ok\" role=\"status\">Playing on the screen now (it takes a few seconds).</p>")
        }
        Notice::PasswordChanged => h.push_str(
            "<p class=\"notice ok\" role=\"status\">Password changed. Everyone else who was logged in has to \
             log in again.</p>",
        ),
    }
    h.push_str("<form method=\"post\" action=\"/admin\" id=\"settings\">");

    // Leagues: followed first, in ticker order, then the rest.
    h.push_str(
        "<section><h2>Leagues</h2><p class=\"hint\">Tick the leagues to follow. Drag (or use the numbers) \
         to set the ticker order.</p><ol class=\"leagues\" id=\"leagues\">",
    );
    let mut ordered: Vec<&LeagueInfo> =
        s.leagues.iter().filter_map(|id| v.leagues.iter().find(|l| &l.id == id)).collect();
    ordered.extend(v.leagues.iter().filter(|l| !s.leagues.contains(&l.id)));
    for (i, l) in ordered.iter().enumerate() {
        let id = esc(l.id.as_str());
        let _ = write!(
            h,
            "<li><span class=\"grip\" aria-hidden=\"true\">⋮⋮</span>\
             <label><input type=\"checkbox\" name=\"league\" value=\"{id}\"{}> {}</label>\
             <input class=\"order\" type=\"number\" name=\"order_{id}\" value=\"{}\" min=\"1\" aria-label=\"Order for {}\">\
             </li>",
            checked(s.leagues.contains(&l.id)),
            esc(&l.name),
            i + 1,
            esc(&l.name),
        );
    }
    h.push_str("</ol></section>");

    // Favorites: every team in the followed leagues (today's teams until the
    // lists load), plus saved favorites from leagues no longer followed.
    h.push_str(
        "<section><h2>Favorite teams</h2><p class=\"hint\">Pick your teams: their games come first, their \
         standings ride on the ticker, and their big plays can take over the screen. Every team in the leagues \
         you follow is here (a minute after you add a league).</p>",
    );
    let mut groups: TeamGroups = s
        .leagues
        .iter()
        .map(|league| {
            let teams =
                v.teams.iter().filter(|t| &t.league == league).map(|t| (t.id.clone(), t.name.clone())).collect();
            (league_name(v.leagues, league).to_owned(), teams)
        })
        .filter(|(_, teams): &(String, Vec<(TeamId, String)>)| !teams.is_empty())
        .collect();
    // Favorites in leagues no longer followed (or not loaded yet) stay ticked.
    let offstage: Vec<(TeamId, String)> =
        s.favorites.iter().filter(|f| !v.teams.iter().any(|t| &t.id == *f)).map(|f| (f.clone(), f.0.clone())).collect();
    if !offstage.is_empty() {
        groups.push(("Other saved favorites".into(), offstage));
    }
    let any = !groups.is_empty();
    if any {
        h.push_str(&team_picker(&groups, &s.favorites));
    }
    if !any {
        h.push_str("<p class=\"empty\">No games loaded yet. Check back once the scoreboard has data.</p>");
    }
    h.push_str("</section>");

    // Takeovers.
    h.push_str("<section><h2>Big-play takeovers</h2><div class=\"choices\">");
    for (value, policy, label) in [
        ("all", TakeoverPolicy::All, "Every touchdown, home run and goal"),
        ("favorites", TakeoverPolicy::Favorites, "Favorites only (others flash the ticker)"),
        ("off", TakeoverPolicy::Off, "Off (just flash the ticker)"),
    ] {
        let _ = write!(
            h,
            "<label><input type=\"radio\" name=\"takeovers\" value=\"{value}\"{}> {label}</label>",
            checked(s.takeovers == policy)
        );
    }
    h.push_str("</div></section>");

    // Widgets: pick a layout, then a widget per slot. The slot boxes follow
    // the chosen layout with CSS alone (:has); dragging one onto another
    // swaps them (admin.js). The selects are what gets submitted.
    h.push_str("<section class=\"widgets\"><h2>Widgets</h2><div class=\"layouts\">");
    for layout in WidgetLayout::ALL {
        let _ = write!(
            h,
            "<label class=\"layout-choice\"><input type=\"radio\" name=\"widget_layout\" value=\"{id}\"{}>\
             <span class=\"mini mini-{id}\">{}</span>{}</label>",
            checked(s.display.widget_layout == layout),
            "<i></i>".repeat(layout.slots()),
            layout.label(),
            id = layout.id(),
        );
    }
    h.push_str(
        "</div><p class=\"hint js-only\">Drag a slot onto another to swap them.</p><div class=\"slots\" id=\"slots\">",
    );
    let fill = [WidgetKind::GameOfTheDay, WidgetKind::Scores, WidgetKind::Standings];
    let most = WidgetLayout::ALL.iter().map(|l| l.slots()).max().unwrap_or(1);
    for slot in 0..most {
        let current = s.widgets.get(slot).cloned().unwrap_or_else(|| fill[slot % fill.len()].into());
        let _ = write!(
            h,
            "<div class=\"slot\"><span>Slot {}</span><select class=\"kind\" name=\"widget_{slot}\" aria-label=\"Slot {} widget\">",
            slot + 1,
            slot + 1
        );
        for kind in WidgetKind::all() {
            let _ = write!(
                h,
                "<option value=\"{}\"{}>{}</option>",
                kind.id(),
                selected(current.kind == *kind),
                kind.label()
            );
        }
        h.push_str("</select>");
        // One option picker per kind; CSS shows the one for the chosen kind.
        for kind in WidgetKind::all() {
            let choices = kind.choices(s);
            if choices.is_empty() {
                continue;
            }
            let _ = write!(
                h,
                "<select class=\"opt opt-{id}\" name=\"widget_{slot}_{id}\" aria-label=\"Slot {} {}\">",
                slot + 1,
                kind.label(),
                id = kind.id()
            );
            let chosen = (current.kind == *kind).then_some(current.option.as_deref()).flatten();
            for (value, label) in choices {
                let _ = write!(
                    h,
                    "<option value=\"{}\"{}>{}</option>",
                    esc(&value),
                    selected(chosen == Some(value.as_str())),
                    esc(&label)
                );
            }
            h.push_str("</select>");
        }
        h.push_str("</div>");
    }
    h.push_str("</div></section>");

    // Theme: pick a look, then optionally change its colors.
    let t = &s.display.theme;
    h.push_str(
        "<section id=\"look\"><h2>Look</h2><p class=\"hint\">How the crawl and widgets look. The ticker \
         stays LED in every look. Save to see it on the screen.</p><div class=\"themes\">",
    );
    for style in Style::ALL {
        let _ = write!(
            h,
            "<label class=\"theme-choice\"><input type=\"radio\" name=\"theme_style\" value=\"{id}\"{}>\
             <img src=\"/admin/theme/{id}.svg\" alt=\"\" width=\"320\" height=\"180\">\
             <strong>{}</strong><small>{}</small></label>",
            checked(t.style == style),
            style.label(),
            style.blurb(),
            id = style.id(),
        );
    }
    let _ = write!(
        h,
        "</div><div class=\"choices\"><label><input type=\"checkbox\" name=\"team_colors\"{}> \
         Use each team's colors (off: everything stays in the look's own colors)</label>\
         <label><input type=\"checkbox\" name=\"provider_logos\"{}> Show team logos from ESPN, the scores \
         service</label></div><p class=\"hint\">Off by default. When on, Marqueet downloads the logos of the \
         teams playing today from ESPN and shows them on the ticker and widgets. The logos belong to the \
         teams; logos you add yourself (below) always win.</p>\
         <details class=\"custom\"{}><summary>Make your own colors</summary>\
         <p class=\"hint\">Change any color, then save. Picking a different look above starts over \
         from that look's colors.</p>",
        checked(t.team_colors),
        checked(s.provider_logos),
        if t.is_preset() { "" } else { " open" },
    );
    if !t.is_preset() {
        h.push_str(
            "<img class=\"current\" src=\"/admin/theme/current.svg\" alt=\"Your colors\" width=\"320\" height=\"180\">",
        );
    }
    h.push_str("<div class=\"grid colors\">");
    for (role, label) in Palette::ROLES {
        let _ = write!(
            h,
            "<label><input type=\"color\" name=\"color_{role}\" value=\"{}\"> {label}</label>",
            t.palette.get(role).unwrap_or(marqueet_core::Rgb::BLACK)
        );
    }
    h.push_str("</div>");
    for (what, on) in t.palette.hard_to_read() {
        let _ = write!(h, "<p class=\"warn\">{what} on {on} may be hard to read from across the room.</p>");
    }
    let _ = write!(
        h,
        "<div class=\"choices\"><label><input type=\"checkbox\" name=\"theme_reset\"> Go back to the look's own colors</label></div>\
         <h3>Share</h3><p class=\"hint\">Copy this code to share your look, or paste someone else's and save.</p>\
         <label class=\"wide\">Your code <input class=\"code\" readonly value=\"{}\" aria-label=\"Your theme code\"></label>\
         <label class=\"wide\">Use a code <input class=\"code\" name=\"theme_code\" placeholder=\"broadcast:0c131b,…\" \
         autocomplete=\"off\" spellcheck=\"false\"></label></details></section>",
        esc(&t.code()),
    );

    // Spotlight: one game fills the widget area.
    let _ = write!(
        h,
        "<section id=\"spotlight\"><h2>Spotlight</h2><p class=\"hint\">One game fills the bottom of the \
         screen: the big score, down and distance, the last play and the game's stats. The ticker keeps \
         running.</p>\
         <div class=\"choices\"><label><input type=\"checkbox\" name=\"spotlight_auto\"{}> Automatically, \
         when only one game is on</label>\
         <label><input type=\"checkbox\" name=\"spotlight_primetime\"{}> Primetime football: the only \
         game on in its league (like Thursday night), even with other sports on</label>\
         <label><input type=\"checkbox\" name=\"spotlight_favorites\"{}> When one of my teams is playing, \
         even with other games on</label>\
         <label><input type=\"checkbox\" name=\"spotlight_tracker\"{}> The live play tracker: baseball's at-bat \
         (pitch by pitch) and football's drive (it takes turns with the stats)</label></div>\
         <label class=\"wide\">Watch a game <select name=\"spotlight_game\" aria-label=\"Watch a game\">\
         <option value=\"\">No, just automatic</option>",
        checked(s.spotlight.auto),
        checked(s.spotlight.primetime),
        checked(s.spotlight.favorites),
        checked(s.spotlight.tracker),
    );
    for (id, label) in v.games {
        let _ = write!(
            h,
            "<option value=\"{}\"{}>{}</option>",
            esc(&id.0),
            selected(s.spotlight.game.as_ref() == Some(id)),
            esc(label)
        );
    }
    if let Some(id) = s.spotlight.game.as_ref().filter(|id| !v.games.iter().any(|(g, _)| g == *id)) {
        let _ = write!(h, "<option value=\"{}\" selected>A game that's no longer on</option>", esc(&id.0));
    }
    h.push_str("</select></label></section>");

    // Display look.
    let d = &s.display;
    let _ = write!(
        h,
        "<section><h2>Display</h2><div class=\"grid\">\
         <label class=\"wide\">Ticker size: how much of the screen the ticker and crawl take (the widgets get \
         the rest) <input type=\"range\" name=\"ticker_ratio\" value=\"{}\" min=\"0.25\" max=\"0.6\" step=\"0.01\" \
         aria-label=\"Ticker size\"></label>\
         <label>LED color <input type=\"color\" name=\"led_color\" value=\"{}\"></label>\
         <label>Ticker rows <input type=\"number\" name=\"ticker_rows\" value=\"{}\" min=\"9\" max=\"48\"></label>\
         <label>Ticker speed <input type=\"number\" name=\"ticker_speed\" value=\"{}\" min=\"1\" max=\"200\" step=\"any\"></label>\
         <label>Crawl speed <input type=\"number\" name=\"crawl_speed\" value=\"{}\" min=\"1\" max=\"200\" step=\"any\"></label>\
         <label>Glow <input type=\"range\" name=\"glow\" value=\"{}\" min=\"0\" max=\"2\" step=\"0.05\"></label>\
         <label>Flicker <input type=\"range\" name=\"flicker\" value=\"{}\" min=\"0\" max=\"1\" step=\"0.05\"></label>\
         </div><div class=\"choices inline\">\
         <label><input type=\"radio\" name=\"scroll_mode\" value=\"stepped\"{}> Stepped (like a real sign)</label>\
         <label><input type=\"radio\" name=\"scroll_mode\" value=\"smooth\"{}> Smooth</label></div>\
         <h3>Screen</h3><div class=\"row\"><label>Resolution <select name=\"resolution\" aria-label=\"Resolution\">{}</select>\
         </label><label><input type=\"radio\" name=\"max_fps\" value=\"60\"{}> 60 frames a second (smoothest)</label>\
         <label><input type=\"radio\" name=\"max_fps\" value=\"30\"{}> 30 (cooler)</label></div>\
         <p class=\"hint\">A Raspberry Pi 4 can't fill a 4K TV smoothly: Automatic draws at 1080p and scales it up. \
         Pick a lower resolution if scrolling stutters.</p>{}\
         <div class=\"choices\"><label><input type=\"checkbox\" name=\"show_odds\"{}> Show betting lines (the spread \
         and over/under, like a broadcast)</label></div><p class=\"hint\">Off by default. For information only: \
         with upcoming games in the crawl and in the spotlight. Marqueet doesn't link to sportsbooks or take \
         bets.</p></section>",
        d.ticker_ratio,
        d.led_color,
        d.ticker_rows,
        d.ticker_speed,
        d.crawl_speed,
        d.glow,
        d.flicker,
        checked(d.scroll_mode == ScrollMode::Stepped),
        checked(d.scroll_mode == ScrollMode::Smooth),
        marqueet_core::config::Resolution::ALL
            .iter()
            .map(|r| format!("<option value=\"{}\"{}>{}</option>", r.id(), selected(*r == d.resolution), r.label()))
            .collect::<String>(),
        checked(d.max_fps >= 60),
        checked(d.max_fps < 60),
        match v.tv_output {
            Some(t) => {
                let mut sizes: Vec<String> = Vec::new();
                for m in &t.modes {
                    let d = crate::tv_output::describe(m);
                    if !sizes.contains(&d) {
                        sizes.push(d);
                    }
                }
                format!(
                    "<p class=\"hint\">Your TV is getting {}. It offers: {}.</p>",
                    esc(&crate::tv_output::describe(&t.applied)),
                    esc(&sizes.join(", "))
                )
            }
            None => String::new(),
        },
        checked(s.show_odds),
    );

    // Weather.
    let w = &s.weather;
    let _ = write!(
        h,
        "<section><h2>Weather</h2><p class=\"hint\">For the weather widget. Forecasts come from \
         <a href=\"https://open-meteo.com\">Open-Meteo</a> (CC BY 4.0); the location is sent to them only \
         while the weather is on screen (ticker or widget).</p>\
         <label class=\"wide\">Location <input name=\"location\" value=\"{}\" \
         placeholder=\"City, or latitude, longitude\" autocomplete=\"off\"></label>\
         <div class=\"choices inline\">\
         <label><input type=\"radio\" name=\"units\" value=\"fahrenheit\"{}> °F, mph</label>\
         <label><input type=\"radio\" name=\"units\" value=\"celsius\"{}> °C, km/h</label></div>\
         <div class=\"choices\"><label><input type=\"checkbox\" name=\"weather_ticker\"{}> \
         Show the weather in the ticker (with a heads-up when rain or snow is coming)</label>\
         <label><input type=\"checkbox\" name=\"weather_alerts\"{}> Severe weather alerts (US only, from the \
         National Weather Service): on the ticker while in effect, and warnings take over the screen</label></div></section>",
        esc(w.place.as_ref().map_or("", |p| p.name.as_str())),
        checked(w.units == Units::Fahrenheit),
        checked(w.units == Units::Celsius),
        checked(w.ticker),
        checked(w.alerts),
    );

    // Time zone and quiet hours.
    let _ = write!(
        h,
        "<section><h2>Time</h2><label class=\"wide\">Time zone \
         <input name=\"time_zone\" list=\"zones\" value=\"{}\" placeholder=\"Your town's (or the device's)\" \
         autocomplete=\"off\" spellcheck=\"false\"></label><datalist id=\"zones\">",
        esc(s.time_zone.as_deref().unwrap_or("")),
    );
    for z in v.zones {
        let _ = write!(h, "<option value=\"{}\">", esc(z));
    }
    h.push_str("</datalist>");
    let q = s.quiet_hours;
    let _ = write!(
        h,
        "<h3>Night mode</h3><div class=\"row\">\
         <label><input type=\"checkbox\" name=\"quiet_enabled\"{}> Night mode</label>\
         <label>Screen <select name=\"quiet_look\" aria-label=\"Night mode screen\">\
         <option value=\"dim\"{}>Dim</option><option value=\"black\"{}>Black</option></select></label>\
         <label>from <input type=\"time\" name=\"quiet_from\" value=\"{}\"></label>\
         <label>to <input type=\"time\" name=\"quiet_to\" value=\"{}\"></label></div></section>",
        checked(q.is_some()),
        selected(q.is_none_or(|q| q.dim)),
        selected(q.is_some_and(|q| !q.dim)),
        q.map_or("23:00".into(), |q| q.from.format("%H:%M").to_string()),
        q.map_or("07:00".into(), |q| q.to.format("%H:%M").to_string()),
    );

    h.push_str("<div class=\"actions\"><button type=\"submit\" class=\"primary\">Save</button></div></form>");

    team_art_section(&mut h, v);
    test_takeovers_section(&mut h, v);

    // Fantasy (own forms: they act at once).
    h.push_str(
        "<section id=\"fantasy\"><h2>Fantasy</h2><p class=\"hint\">Follow your Sleeper teams: your matchup \
         lives on the ticker, and a touchdown by one of your starters says so on the screen.</p>",
    );
    if !v.fantasy.is_empty() {
        h.push_str("<table class=\"feeds\"><thead><tr><th>League</th><th>Team</th><th>This week</th><th></th></tr></thead><tbody>");
        for f in v.fantasy {
            let _ = write!(
                h,
                "<tr><td>{}</td><td>{}</td><td>{}</td><td><form method=\"post\" action=\"/admin/fantasy/remove\">\
                 <input type=\"hidden\" name=\"league_id\" value=\"{}\"><input type=\"hidden\" name=\"roster_id\" value=\"{}\">\
                 <button>Remove</button></form></td></tr>",
                esc(&f.league),
                esc(&f.team),
                esc(&f.status),
                esc(&f.league_id),
                f.roster_id,
            );
        }
        h.push_str("</tbody></table>");
    }
    let _ = write!(
        h,
        "<form method=\"post\" action=\"/admin/fantasy/find\" class=\"row new-feed\"><label>Sleeper username \
         <input name=\"username\" required value=\"{}\" autocomplete=\"off\" spellcheck=\"false\"></label>\
         <button>Find leagues</button></form>",
        esc(v.fantasy_search.map_or("", |s| s.user.display_name.as_str())),
    );
    if let Some(search) = v.fantasy_search {
        if search.leagues.is_empty() {
            let _ = write!(h, "<p class=\"empty\">{} has no leagues this season.</p>", esc(&search.user.display_name));
        }
        for (league, teams) in &search.leagues {
            let _ = write!(
                h,
                "<form method=\"post\" action=\"/admin/fantasy/add\" class=\"row new-feed\">\
                 <input type=\"hidden\" name=\"league_id\" value=\"{}\"><input type=\"hidden\" name=\"league\" value=\"{}\">\
                 <label><strong>{}</strong> <select name=\"roster_id\">",
                esc(&league.id),
                esc(&league.name),
                esc(&league.name),
            );
            for t in teams {
                let mine = t.owner_id.as_deref() == Some(search.user.id.as_str());
                let _ = write!(h, "<option value=\"{}\"{}>{}</option>", t.roster_id, selected(mine), esc(&t.name));
            }
            h.push_str("</select></label><button>Follow</button></form>");
        }
    }
    h.push_str("</section>");

    // Feeds (their own forms: they act at once, not on Save).
    h.push_str(
        "<section id=\"feeds\"><h2>Feeds</h2><p class=\"hint\">Let your own scripts put things on the \
         sign: stock prices, server status, the doorbell. Each feed gets a token; send it as \
         <code>Authorization: Bearer …</code>. See <code>docs/FEEDS.md</code> for the format.</p>",
    );
    if !v.feeds.is_empty() {
        h.push_str(
            "<table class=\"feeds\"><thead><tr><th>Feed</th><th>On screen</th><th></th><th></th></tr></thead><tbody>",
        );
        for f in v.feeds {
            let state = match f.expires_at {
                Some(t) if t > v.now => {
                    let mins = (t - v.now).num_minutes().max(1);
                    format!(
                        "<span class=\"good\">{} segment{}</span>, {mins} min left",
                        f.segments,
                        if f.segments == 1 { "" } else { "s" }
                    )
                }
                _ => "idle".into(),
            };
            let _ = write!(
                h,
                "<tr><td>{0}</td><td>{state}</td><td>\
                 <form method=\"post\" action=\"/admin/feeds/token\"><input type=\"hidden\" name=\"name\" value=\"{0}\">\
                 <button>New token</button></form></td><td>\
                 <form method=\"post\" action=\"/admin/feeds/revoke\"><input type=\"hidden\" name=\"name\" value=\"{0}\">\
                 <button>Revoke</button></form></td></tr>",
                esc(&f.name),
            );
        }
        h.push_str("</tbody></table>");
    }
    h.push_str(
        "<form method=\"post\" action=\"/admin/feeds\" class=\"row new-feed\"><label>New feed \
         <input name=\"name\" required pattern=\"[a-z0-9_\\-]{1,32}\" placeholder=\"stocks\" \
         title=\"1-32 of a-z, 0-9, - and _\"></label><button>Create</button></form>",
    );
    let (name, token) = match &v.notice {
        Notice::NewToken { feed, token } => (feed.as_str(), token.as_str()),
        _ => (v.feeds.first().map_or("stocks", |f| f.name.as_str()), "TOKEN"),
    };
    let _ = write!(
        h,
        "<details><summary>Example</summary><pre>curl -X POST http://{host}/api/feeds/{name} \\\n  \
         -H 'Authorization: Bearer {token}' \\\n  -H 'Content-Type: application/json' \\\n  \
         -d '{{\"segments\": [{{\"text\": \"AAPL 189.20\", \"detail\": \"+1.2%\", \"color\": \"green\"}}]}}'</pre></details></section>",
        host = esc(v.host),
        name = esc(name),
        token = esc(token),
    );

    // Admin password (its own form).
    h.push_str("<section id=\"password\"><h2>Admin password</h2>");
    if v.can_change_password {
        h.push_str(
            "<form method=\"post\" action=\"/admin/password\" class=\"narrow\">\
             <label>Current password <input type=\"password\" name=\"current\" autocomplete=\"current-password\" required></label>\
             <label>New password <input type=\"password\" name=\"password\" autocomplete=\"new-password\" minlength=\"8\" maxlength=\"128\" required></label>\
             <label>New password again <input type=\"password\" name=\"confirm\" autocomplete=\"new-password\" minlength=\"8\" maxlength=\"128\" required></label>\
             <p class=\"hint\">8 to 128 characters. Everyone else who is logged in will have to log in again.</p>\
             <div class=\"actions\"><button>Change password</button></div></form>",
        );
    } else {
        h.push_str(
            "<p class=\"hint\">This device's password comes from its configuration \
             (<code>MARQUEET_ADMIN_PASSWORD</code>), so change it there.</p>",
        );
    }
    h.push_str("</section>");

    // Status (read-only).
    h.push_str("<section class=\"status\"><h2>Status</h2><table><thead><tr><th>League</th><th>Games</th><th>Last update</th><th>State</th></tr></thead><tbody>");
    for l in v.health {
        let state = if l.stale {
            format!("<span class=\"bad\">stale</span> {}", esc(l.last_error.as_deref().unwrap_or("")))
        } else if l.failures > 0 {
            format!("<span class=\"warn\">retrying ({})</span>", l.failures)
        } else if l.last_success.is_some() {
            "<span class=\"good\">ok</span>".into()
        } else {
            "waiting".into()
        };
        let _ = write!(
            h,
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{state}</td></tr>",
            esc(league_name(v.leagues, &l.id)),
            l.games,
            l.last_success.map_or("never".into(), |t| ago(t, v.now)),
        );
    }
    h.push_str("</tbody></table><h3>Recent plays</h3>");
    if v.alerts.is_empty() {
        h.push_str("<p class=\"empty\">Nothing yet.</p>");
    } else {
        h.push_str("<ul class=\"alerts\">");
        for a in v.alerts.iter().take(12) {
            let kind = if a.level == AlertLevel::Takeover { "takeover" } else { "flash" };
            let _ = write!(
                h,
                "<li><time>{}</time> <strong>{}</strong> {} <span class=\"tag\">{kind}</span></li>",
                a.created_at.with_timezone(&v.tz).format("%-I:%M %p"),
                esc(&a.title),
                esc(a.detail.as_deref().unwrap_or("")),
            );
        }
        h.push_str("</ul>");
    }
    h.push_str("</section></main></body></html>");
    h
}

pub fn login(error: bool) -> String {
    login_page(error.then_some("Wrong password."))
}

/// The login page with a message (e.g. "Too many tries").
pub fn login_message(message: &str) -> String {
    login_page(Some(message))
}

fn login_page(error: Option<&str>) -> String {
    let mut h = head("Marqueet login");
    h.push_str(BRAND);
    h.push_str("</header><main class=\"narrow\"><form method=\"post\" action=\"/login\"><h2>Log in</h2>");
    if let Some(e) = error {
        let _ = write!(h, "<p class=\"notice err\" role=\"alert\">{}</p>", esc(e));
    }
    h.push_str(
        "<label>Admin password <input type=\"password\" name=\"password\" autocomplete=\"current-password\" \
         autofocus required></label><div class=\"actions\"><button class=\"primary\">Log in</button></div>\
         </form></main></body></html>",
    );
    h
}

/// First boot: the code from the screen and a new admin password.
pub fn setup(error: Option<&str>) -> String {
    let mut h = head("Set up Marqueet");
    h.push_str(BRAND);
    h.push_str(
        "</header><main class=\"narrow\"><form method=\"post\" action=\"/setup\"><h2>Set up Marqueet</h2>\
         <p class=\"hint\">Enter the 6-digit code shown on the Marqueet screen, then choose the password \
         you'll use to manage it from this and other devices.</p>",
    );
    if let Some(e) = error {
        let _ = write!(h, "<p class=\"notice err\" role=\"alert\">{}</p>", esc(e));
    }
    h.push_str(
        "<label>Code from the screen <input name=\"code\" inputmode=\"numeric\" pattern=\"[0-9]{6}\" \
         maxlength=\"6\" autocomplete=\"one-time-code\" autofocus required></label>\
         <label>New admin password <input type=\"password\" name=\"password\" minlength=\"8\" maxlength=\"128\" \
         autocomplete=\"new-password\" required></label>\
         <label>Password again <input type=\"password\" name=\"confirm\" minlength=\"8\" maxlength=\"128\" \
         autocomplete=\"new-password\" required></label>\
         <div class=\"actions\"><button class=\"primary\">Set up</button></div></form></main></body></html>",
    );
    h
}

/// "Your team colors and logos": what the person added, a form to add or
/// change one team, and team pack import/export. Its own forms (uploads), so
/// it sits outside the settings form.
fn team_art_section(h: &mut String, v: &View<'_>) {
    h.push_str(
        "<section id=\"teams\"><h2>Your team colors and logos</h2><p class=\"hint\">Marqueet doesn't come \
         with any team logos. If you have images you're allowed to use, add them here: they show next to the \
         team on the ticker and in the widgets, and stay on this device. Colors set here replace the ones from \
         the scores everywhere.</p>",
    );
    if !v.team_art.is_empty() {
        h.push_str("<table class=\"feeds team-art\"><thead><tr><th>Logo</th><th>Team</th><th>Colors</th><th></th></tr></thead><tbody>");
        for (team, art) in v.team_art {
            let logo = if art.logo.is_some() {
                format!(
                    "<img src=\"/admin/teams/logo?team={}\" alt=\"\" width=\"40\" height=\"40\">",
                    esc(&form_urlencoded::byte_serialize(team.0.as_bytes()).collect::<String>())
                )
            } else {
                "–".into()
            };
            let colors = art.colors.map_or_else(
                || "From the scores".to_owned(),
                |c| {
                    let swatch = |rgb: marqueet_core::Rgb| {
                        format!("<input type=\"color\" value=\"{rgb}\" disabled aria-label=\"{rgb}\">")
                    };
                    format!("{}{}", swatch(c.primary), c.secondary.map(swatch).unwrap_or_default())
                },
            );
            let mut words: Vec<String> = art.words.values().map(|w| esc(&w.headline)).collect();
            if let Some(a) = &art.art {
                let (w, h) = a.size();
                let what = if a.frames.len() > 1 { format!("{} frames", a.frames.len()) } else { "still".into() };
                words.push(format!("LED art {w}x{h}, {what}"));
            }
            let colors = if words.is_empty() { colors } else { format!("{colors}<br>{}", words.join(" · ")) };
            let _ = write!(
                h,
                "<tr><td>{logo}</td><td>{}</td><td class=\"swatches\">{colors}</td><td><form method=\"post\" \
                 action=\"/admin/teams/remove\"><input type=\"hidden\" name=\"team\" value=\"{}\">\
                 <button>Remove</button></form></td></tr>",
                esc(&art.label),
                esc(&team.0),
            );
        }
        h.push_str("</tbody></table>");
    }
    h.push_str(
        "<form method=\"post\" action=\"/admin/teams\" enctype=\"multipart/form-data\" class=\"team-form\">\
         <label class=\"wide\">Team <select name=\"team\" required><option value=\"\">Pick a team…</option>",
    );
    for league in v.leagues.iter().filter(|l| v.teams.iter().any(|t| t.league == l.id)) {
        let _ = write!(h, "<optgroup label=\"{}\">", esc(&league.name));
        for t in v.teams.iter().filter(|t| t.league == league.id) {
            let _ = write!(h, "<option value=\"{}\">{}</option>", esc(&t.id.0), esc(&t.name));
        }
        h.push_str("</optgroup>");
    }
    h.push_str(
        "</select></label>\
         <div class=\"row\"><label><input type=\"checkbox\" name=\"custom_colors\"> Use my colors</label>\
         <label>Main <input type=\"color\" name=\"primary\" value=\"#c8102e\"></label>\
         <label>Second <input type=\"color\" name=\"secondary\" value=\"#ffffff\"></label></div>\
         <label class=\"wide\">Logo <input type=\"file\" name=\"logo\" accept=\"image/png\"></label>\
         <p class=\"hint\">A PNG, ideally square with a see-through background. It's shrunk to fit.</p>\
         <div class=\"row\"><label><input type=\"checkbox\" name=\"remove_logo\"> Remove this team's logo</label></div>",
    );
    takeover_words_fields(h);
    h.push_str(
        "<div class=\"row\"><button class=\"primary\">Save team</button></div></form>\
         <h3>Team packs</h3><p class=\"hint\">One file with colors and logos for many teams, to move them \
         between devices or share with friends. <a href=\"/admin/teams/pack.json?all=1\">Download a blank pack</a> \
         listing every team you follow, fill it in, and import it; or <a href=\"/admin/teams/pack.json\">download \
         yours</a>.</p>\
         <form method=\"post\" action=\"/admin/teams/import\" enctype=\"multipart/form-data\" class=\"row\">\
         <label>Team pack <input type=\"file\" name=\"pack\" accept=\"application/json,.json\" required></label>\
         <button>Import</button></form></section>",
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use marqueet_core::sports::Sport;

    fn info(id: &str, name: &str) -> LeagueInfo {
        LeagueInfo { id: LeagueId::new(id), sport: Sport::Football, name: name.into() }
    }

    static NO_ART: TeamArtMap = TeamArtMap::new();

    fn view<'a>(settings: &'a Settings, leagues: &'a [LeagueInfo], teams: &'a [TeamChoice]) -> View<'a> {
        View {
            settings,
            leagues,
            teams,
            health: &[],
            alerts: &[],
            zones: &[],
            feeds: &[],
            fantasy: &[],
            fantasy_search: None,
            team_art: &NO_ART,
            games: &[],
            playing: &[],
            tv_output: None,
            host: "marqueet.local:7878",
            notice: Notice::None,
            remote: false,
            can_change_password: true,
            tz: FixedOffset::east_opt(0).unwrap(),
            now: Utc::now(),
        }
    }

    #[test]
    fn test_takeover_buttons_are_their_own_form() {
        let settings = Settings::default();
        let playing = [(TeamId("espn:nfl:1".into()), "NFL · <Blizzard>".into())];
        let h = render(&View { playing: &playing, ..view(&settings, &[], &[]) });
        let section = &h[h.find("id=\"test-takeover\"").unwrap()..];
        assert!(section.contains("action=\"/admin/takeover/test\""));
        for kind in marqueet_core::test_alerts::TestKind::ALL {
            assert!(section.contains(&format!("value=\"{}\"", kind.id())), "{kind:?}");
        }
        assert!(section.contains("NFL · &lt;Blizzard&gt;"), "escaped");
        let settings_form = &h[h.find("id=\"settings\"").unwrap()..];
        let end = settings_form.find("</form>").unwrap();
        assert!(!settings_form[..end].contains("/admin/takeover/test"), "not nested in the settings form");
    }

    #[test]
    fn escapes_everything_interpolated() {
        assert_eq!(esc(r#"<a href="x">'&'</a>"#), "&lt;a href=&quot;x&quot;&gt;&#39;&amp;&#39;&lt;/a&gt;");
        let settings = Settings {
            leagues: vec![LeagueId::new("nfl")],
            favorites: vec![TeamId("\"><script>x</script>".into())],
            ..Settings::default()
        };
        let leagues = [info("nfl", "N<F>L")];
        let teams = [TeamChoice { id: TeamId("espn:nfl:1".into()), league: LeagueId::new("nfl"), name: "<b>".into() }];
        let mut v = view(&settings, &leagues, &teams);
        v.notice = Notice::Error("<img src=x>".into());
        let html = render(&v);
        assert!(!html.contains("<script>x") && !html.contains("<b>") && !html.contains("<img src=x"));
        assert!(html.contains("N&lt;F&gt;L") && html.contains("&lt;img src=x&gt;"));
    }

    #[test]
    fn feed_tokens_show_once() {
        let settings = Settings::default();
        let feeds = [FeedRow { name: "stocks".into(), segments: 0, expires_at: None }];
        let mut v = view(&settings, &[], &[]);
        v.feeds = &feeds;
        let html = render(&v);
        assert!(html.contains("New token") && html.contains("Bearer TOKEN"), "no token on an ordinary view");
        v.notice = Notice::NewToken { feed: "stocks".into(), token: "abc123".into() };
        let html = render(&v);
        assert!(html.contains("<code class=\"token\">abc123</code>") && html.contains("Bearer abc123"));
    }

    #[test]
    fn password_form_only_when_it_can_change() {
        let settings = Settings::default();
        let mut v = view(&settings, &[], &[]);
        assert!(render(&v).contains("action=\"/admin/password\""));
        v.can_change_password = false;
        let html = render(&v);
        assert!(!html.contains("action=\"/admin/password\"") && html.contains("MARQUEET_ADMIN_PASSWORD"));
    }

    #[test]
    fn followed_leagues_come_first_in_order() {
        let settings = Settings { leagues: vec![LeagueId::new("mlb"), LeagueId::new("nfl")], ..Settings::default() };
        let leagues = [info("nfl", "NFL"), info("nhl", "NHL"), info("mlb", "MLB")];
        let html = render(&view(&settings, &leagues, &[]));
        let at = |needle: &str| html.find(needle).unwrap();
        assert!(at("value=\"mlb\" checked") < at("value=\"nfl\" checked"));
        assert!(at("value=\"nfl\" checked") < at("value=\"nhl\">"), "unfollowed leagues are unchecked, last");
        assert!(html.contains("name=\"order_mlb\" value=\"1\"") && html.contains("name=\"order_nhl\" value=\"3\""));
    }

    #[test]
    fn favorites_not_playing_stay_ticked() {
        let settings = Settings {
            leagues: vec![LeagueId::new("nfl")],
            favorites: vec![TeamId("espn:nfl:2".into()), TeamId("espn:nfl:9".into())],
            ..Settings::default()
        };
        let leagues = [info("nfl", "NFL")];
        let teams =
            [TeamChoice { id: TeamId("espn:nfl:2".into()), league: LeagueId::new("nfl"), name: "Blizzard".into() }];
        let html = render(&view(&settings, &leagues, &teams));
        assert!(html.contains("value=\"espn:nfl:2\" data-league=\"NFL\" checked> Blizzard"));
        assert!(html.contains("Other saved favorites") && html.contains("value=\"espn:nfl:9\" data-league"));
        assert!(html.contains("value=\"espn:nfl:9\" data-league=\"Other saved favorites\" checked"));
    }

    #[test]
    fn team_picker_shows_picks_on_top_and_one_search() {
        let a = TeamId("a".into());
        let groups: TeamGroups = vec![
            ("NFL".into(), vec![(a.clone(), "Buffalo <Blizzard>".into()), (TeamId("b".into()), "Dallas".into())]),
            ("College".into(), vec![(a.clone(), "Buffalo <Blizzard>".into())]),
        ];
        let html = team_picker(&groups, std::slice::from_ref(&a));
        assert!(html.contains("<label class=\"chip\" for=\"fav-1\">Buffalo &lt;Blizzard&gt; <small>NFL</small>"));
        assert_eq!(html.matches("class=\"chip\"").count(), 1, "one tag per team");
        assert_eq!(html.matches("value=\"a\"").count(), 1, "one checkbox per team, even in two leagues");
        assert_eq!(html.matches("team-search").count(), 1, "one search for every league");
        assert!(html.contains("data-total=\"2 teams\">1 picked"));
        assert!(!html.contains("<details class=\"teams\" open"), "leagues start folded");
        let none = team_picker(&groups, &[]);
        assert!(none.contains("No teams picked yet."));
    }

    #[test]
    fn form_round_trips_through_the_parser() {
        // Every value the page renders must parse back to the same settings.
        let settings = Settings {
            leagues: vec![LeagueId::new("nfl"), LeagueId::new("mlb")],
            quiet_hours: Some(marqueet_core::settings::QuietHours {
                from: chrono::NaiveTime::from_hms_opt(23, 30, 0).unwrap(),
                to: chrono::NaiveTime::from_hms_opt(6, 0, 0).unwrap(),
                dim: false,
            }),
            time_zone: Some("America/Chicago".into()),
            ..Settings::default()
        }
        .sanitized();
        let leagues = [info("nfl", "NFL"), info("mlb", "MLB"), info("nhl", "NHL")];
        let html = render(&view(&settings, &leagues, &[]));
        let pairs = submitted(&html);
        let ids: Vec<LeagueId> = leagues.iter().map(|l| l.id.clone()).collect();
        let parsed = super::super::form::apply(&Settings::default(), &ids, &pairs).unwrap();
        assert_eq!(parsed, settings);
    }

    #[test]
    fn look_section_offers_every_style_and_a_code() {
        let mut settings = Settings { leagues: vec![LeagueId::new("nfl")], ..Settings::default() };
        let html = render(&view(&settings, &[info("nfl", "NFL")], &[]));
        for style in Style::ALL {
            assert!(html.contains(&format!("src=\"/admin/theme/{}.svg\"", style.id())));
        }
        assert!(html.contains("value=\"broadcast\" checked") && html.contains(&esc(&settings.display.theme.code())));
        assert!(
            !html.contains("current.svg") && !html.contains("details class=\"custom\" open"),
            "presets stay folded"
        );
        settings.display.theme.palette.text = settings.display.theme.palette.panel;
        let html = render(&view(&settings, &[info("nfl", "NFL")], &[]));
        assert!(html.contains("current.svg") && html.contains("details class=\"custom\" open"));
        assert!(html.contains("Text on Cards may be hard to read"));
    }

    #[test]
    fn team_art_section_lists_art_and_offers_uploads() {
        use marqueet_core::Rgb;
        use marqueet_core::sports::TeamColors;
        use marqueet_core::team_art::{Image, TeamArt};
        let settings = Settings { leagues: vec![LeagueId::new("nfl")], ..Settings::default() };
        let leagues = [info("nfl", "NFL")];
        let teams = [TeamChoice {
            id: TeamId("espn:nfl:2".into()),
            league: LeagueId::new("nfl"),
            name: "Buffalo Blizzard".into(),
        }];
        let html = render(&view(&settings, &leagues, &teams));
        assert!(html.contains("doesn't come with any team logos") && html.contains("enctype=\"multipart/form-data\""));
        assert!(html.contains("<optgroup label=\"NFL\"><option value=\"espn:nfl:2\">Buffalo Blizzard</option>"));
        assert!(!html.contains("/admin/teams/logo?"), "nothing added yet");
        let art: TeamArtMap = [(
            TeamId("espn:nfl:2".into()),
            TeamArt {
                label: "Buffalo <Blizzard>".into(),
                colors: Some(TeamColors { primary: Rgb::RED, secondary: None }),
                logo: Image::new(1, 1, vec![0; 4]),
                words: Default::default(),
                art: Default::default(),
            },
        )]
        .into();
        let v = View { team_art: &art, ..view(&settings, &leagues, &teams) };
        let html = render(&v);
        assert!(html.contains("/admin/teams/logo?team=espn%3Anfl%3A2") && html.contains("Buffalo &lt;Blizzard&gt;"));
        assert!(html.contains("value=\"#ff281e\" disabled"));
        let settings_form = &html[html.find("id=\"settings\"").unwrap()..html.find("</form>").unwrap()];
        assert!(!settings_form.contains("/admin/teams"), "upload forms aren't nested in the settings form");
    }

    /// What a browser would submit for the rendered form (enough of HTML for
    /// our own markup: inputs and selects in the settings form).
    fn submitted(html: &str) -> Vec<(String, String)> {
        let form = &html[html.find("id=\"settings\"").unwrap()..html.find("</form>").unwrap()];
        let attr = |tag: &str, name: &str| {
            let key = format!("{name}=\"");
            tag.find(&key).map(|i| {
                let rest = &tag[i + key.len()..];
                rest[..rest.find('"').unwrap()].to_owned()
            })
        };
        let mut out = Vec::new();
        for tag in form.split('<').skip(1) {
            let tag = &tag[..tag.find('>').unwrap_or(tag.len())];
            if tag.starts_with("input") {
                let (Some(name), value) = (attr(tag, "name"), attr(tag, "value")) else { continue };
                let kind = attr(tag, "type").unwrap_or_default();
                if (kind == "checkbox" || kind == "radio") && !tag.contains(" checked") {
                    continue;
                }
                let empty = if kind == "checkbox" || kind == "radio" { "on" } else { "" };
                out.push((name, value.unwrap_or_else(|| empty.into())));
            } else if tag.starts_with("select") {
                let name = attr(tag, "name").unwrap();
                // This select's options only; browsers submit the first when
                // none is marked selected.
                let rest = &form[form.find(tag).unwrap()..];
                let body = &rest[..rest.find("</select>").unwrap()];
                let options: Vec<&str> = body.split("<option").skip(1).collect();
                let opt = options.iter().find(|o| o.contains(" selected")).or(options.first()).unwrap();
                out.push((name, attr(opt, "value").unwrap()));
            }
        }
        out
    }
}
