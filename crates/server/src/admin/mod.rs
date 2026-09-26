//! The admin page: `GET/POST /admin`, `/login`, `/logout`.
//!
//! Server-rendered HTML and one small hand-written script (drag to reorder
//! leagues). No JS toolchain, no third-party front-end code, and the page
//! works with scripting off.

pub mod auth;
pub mod form;
pub mod multipart;
pub mod page;
pub mod password;
pub mod preview;
pub mod teams;
pub mod welcome;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use axum::body::Bytes;
use axum::extract::{ConnectInfo, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE, LOCATION, SET_COOKIE};
use axum::http::{HeaderMap, HeaderValue, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use chrono::Utc;
use marqueet_core::sports::LeagueId;
use marqueet_core::theme::{Style, Theme};

use crate::hub::Hub;
use crate::tz;
use crate::web::AppState;
use auth::{Access, ChangeError, SetupError};
use marqueet_core::fantasy::points;
use page::{FantasyRow, FeedRow, LeagueHealth, Notice, TeamChoice};

use crate::hub::FantasySearch;
use crate::store::FantasyFeed;

const CSS: &str = include_str!("admin.css");
const JS: &str = include_str!("admin.js");
const CSP: &str = "default-src 'none'; style-src 'self'; script-src 'self'; img-src 'self'; \
                   form-action 'self'; frame-ancestors 'none'; base-uri 'none'";

pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(welcome::routes())
        .route("/", get(|| async { redirect("/admin") }))
        .route("/admin", get(show).post(save))
        .route("/admin/admin.css", get(|| async { asset("text/css; charset=utf-8", CSS) }))
        .route("/admin/admin.js", get(|| async { asset("text/javascript; charset=utf-8", JS) }))
        .route("/admin/theme/{file}", get(theme_preview))
        .merge(teams::routes())
        .route("/admin/fantasy/find", axum::routing::post(find_fantasy))
        .route("/admin/fantasy/add", axum::routing::post(add_fantasy))
        .route("/admin/fantasy/remove", axum::routing::post(remove_fantasy))
        .route("/admin/feeds", axum::routing::post(create_feed))
        .route("/admin/feeds/revoke", axum::routing::post(revoke_feed))
        .route("/admin/feeds/token", axum::routing::post(new_feed_token))
        .route("/admin/password", axum::routing::post(change_password))
        .route("/admin/takeover/test", axum::routing::post(test_takeover))
        .route("/setup", get(setup_page).post(setup))
        .route("/login", get(login_page).post(login))
        .route("/logout", axum::routing::post(logout))
}

fn asset(kind: &'static str, body: &'static str) -> Response {
    ([(CONTENT_TYPE, kind), (CACHE_CONTROL, "no-cache")], body).into_response()
}

fn redirect(to: &'static str) -> Response {
    (StatusCode::SEE_OTHER, [(LOCATION, to)]).into_response()
}

/// An HTML page with the admin security headers.
fn html(status: StatusCode, body: String) -> Response {
    let mut res = (status, body).into_response();
    let h = res.headers_mut();
    h.insert(CONTENT_TYPE, HeaderValue::from_static("text/html; charset=utf-8"));
    h.insert("content-security-policy", HeaderValue::from_static(CSP));
    h.insert("x-content-type-options", HeaderValue::from_static("nosniff"));
    // Not "no-referrer": under that, browsers send `Origin: null` on our own
    // form posts, which the cross-site check then refuses. "same-origin"
    // still sends nothing to other sites.
    h.insert("referrer-policy", HeaderValue::from_static("same-origin"));
    h.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    res
}

/// The response for a request that may not see the admin page, if any.
fn deny(access: Access) -> Option<Response> {
    match access {
        Access::Granted => None,
        Access::NeedsLogin => Some(redirect("/login")),
        Access::Setup => Some(redirect("/setup")),
    }
}

/// The Host header, for examples that point back at this server.
fn host(headers: &HeaderMap) -> &str {
    headers.get(axum::http::header::HOST).and_then(|h| h.to_str().ok()).unwrap_or("marqueet.local:7878")
}

fn pairs(body: &[u8]) -> Vec<(String, String)> {
    form_urlencoded::parse(body).into_owned().collect()
}

/// The admin page as `peer` sees it.
fn admin_page(state: &AppState, notice: Notice, peer: SocketAddr, headers: &HeaderMap) -> String {
    admin_page_with(state, notice, peer, headers, None)
}

fn admin_page_with(
    state: &AppState,
    notice: Notice,
    peer: SocketAddr,
    headers: &HeaderMap,
    search: Option<&FantasySearch>,
) -> String {
    let (remote, host) = (!auth::is_local(peer.ip()), host(headers));
    render(&state.hub, notice, remote, host, search, state.auth.can_change_password())
}

/// Every team in each followed league once the lists are in, and today's
/// teams meanwhile (or where a league has no list), by name.
fn known_teams(store: &crate::store::Store) -> Vec<TeamChoice> {
    let mut teams: Vec<TeamChoice> = Vec::new();
    for league in store.leagues() {
        for t in store.teams(league) {
            teams.push(TeamChoice { id: t.id.clone(), league: league.clone(), name: t.name.clone() });
        }
    }
    for g in store.games() {
        for c in [&g.away, &g.home] {
            if !teams.iter().any(|t| t.id == c.team.id) {
                teams.push(TeamChoice {
                    id: c.team.id.clone(),
                    league: g.league.clone(),
                    name: c.team.display_name.clone(),
                });
            }
        }
    }
    teams.sort_by(|a, b| a.name.cmp(&b.name));
    teams
}

fn render(
    hub: &Hub,
    notice: Notice,
    remote: bool,
    host: &str,
    search: Option<&FantasySearch>,
    can_change_password: bool,
) -> String {
    let feeds: Vec<FeedRow> = hub
        .feeds()
        .into_iter()
        .map(|f| FeedRow { segments: f.segments, expires_at: f.expires_at, name: f.name })
        .collect();
    let settings = hub.settings();
    let leagues = hub.supported_leagues();
    let (teams, health) = hub.with_store(|store| {
        let teams = known_teams(store);
        let health: Vec<LeagueHealth> = store
            .leagues()
            .iter()
            .map(|id| {
                let feed = store.feed(id).cloned().unwrap_or_default();
                LeagueHealth {
                    id: id.clone(),
                    games: feed.games.len(),
                    stale: store.is_stale(id),
                    failures: feed.failures,
                    last_error: feed.last_error,
                    last_success: feed.last_success,
                }
            })
            .collect();
        (teams, health)
    });
    let alerts: Vec<_> = hub.recent_alerts().iter().map(|a| (**a).clone()).collect();
    let fantasy: Vec<FantasyRow> = settings
        .fantasy
        .iter()
        .map(|f| {
            let feed = hub.with_store(|s| s.fantasy(&(f.league_id.clone(), f.roster_id)).cloned());
            let status = match feed {
                Some(FantasyFeed { matchup: Some(m), .. }) => match &m.opponent {
                    Some(o) => format!("Week {}: {} to {}", m.week, points(m.me.points), points(o.points)),
                    None => format!("Week {}: {} (bye)", m.week, points(m.me.points)),
                },
                Some(FantasyFeed { last_error: Some(e), .. }) => format!("Couldn't load: {e}"),
                _ => "Loading…".into(),
            };
            FantasyRow {
                league_id: f.league_id.clone(),
                roster_id: f.roster_id,
                league: f.league.clone(),
                team: f.team.clone(),
                status,
            }
        })
        .collect();
    let now = Utc::now();
    page::render(&page::View {
        settings: &settings,
        leagues: &leagues,
        teams: &teams,
        health: &health,
        alerts: &alerts,
        zones: &tz::names(),
        feeds: &feeds,
        fantasy: &fantasy,
        fantasy_search: search,
        team_art: &hub.team_art(),
        games: &hub.with_store(|store| {
            store
                .games()
                .iter()
                .map(|g| {
                    let (status, _) = marqueet_core::widgets::status_text(g, tz::offset(&settings, now), now);
                    let league = marqueet_core::sports::ticker::league_label(g.league.as_str());
                    let label =
                        format!("{league} · {} at {} · {status}", g.away.team.abbreviation, g.home.team.abbreviation);
                    (g.id.clone(), label)
                })
                .collect::<Vec<_>>()
        }),
        tv_output: crate::tv_output::status().as_ref(),
        playing: &hub.with_store(|store| {
            let mut teams: Vec<(marqueet_core::sports::TeamId, String)> = Vec::new();
            for g in store.games() {
                for t in [&g.away.team, &g.home.team] {
                    if !teams.iter().any(|(id, _)| id == &t.id) {
                        let league = marqueet_core::sports::ticker::league_label(g.league.as_str());
                        teams.push((t.id.clone(), format!("{league} · {}", t.display_name)));
                    }
                }
            }
            teams.sort_by_key(|(id, label)| (!settings.favorites.contains(id), label.clone()));
            teams
        }),
        host,
        notice,
        remote,
        can_change_password,
        tz: tz::offset(&settings, now),
        now,
    })
}

async fn show(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Some(denied) = deny(state.auth.check(peer.ip(), &headers)) {
        return denied;
    }
    let notice = match uri.query() {
        Some("saved") => Notice::Saved,
        Some("password") => Notice::PasswordChanged,
        Some("tested") => Notice::Tested,
        _ => Notice::None,
    };
    html(StatusCode::OK, admin_page(&state, notice, peer, &headers))
}

/// `GET /admin/theme/<style>.svg` (a style's own colors) or
/// `/admin/theme/current.svg` (the saved theme): a sketch for the admin page.
async fn theme_preview(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    axum::extract::Path(file): axum::extract::Path<String>,
) -> Response {
    if let Some(denied) = deny(state.auth.check(peer.ip(), &headers)) {
        return denied;
    }
    let theme = match file.strip_suffix(".svg") {
        Some("current") => state.hub.settings().display.theme,
        Some(id) => match Style::from_id(id) {
            Some(style) => Theme::preset(style),
            None => return StatusCode::NOT_FOUND.into_response(),
        },
        None => return StatusCode::NOT_FOUND.into_response(),
    };
    let headers = [
        (CONTENT_TYPE, "image/svg+xml"),
        (CACHE_CONTROL, "no-store"),
        (axum::http::header::HeaderName::from_static("x-content-type-options"), "nosniff"),
    ];
    (headers, preview::svg(&theme)).into_response()
}

async fn save(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if !auth::same_origin(&headers) {
        return html(StatusCode::FORBIDDEN, "cross-site form post refused".into());
    }
    if let Some(denied) = deny(state.auth.check(peer.ip(), &headers)) {
        return denied;
    }
    let hub = &state.hub;
    let current = hub.settings();
    let mut supported: Vec<LeagueId> = hub.supported_leagues().into_iter().map(|l| l.id).collect();
    if supported.is_empty() {
        supported.clone_from(&current.leagues);
    }
    let pairs = pairs(&body);
    let result = async {
        let mut settings = form::apply(&current, &supported, &pairs)?;
        match form::location(&pairs, current.weather.place.as_ref()) {
            form::LocationChange::Keep => {}
            form::LocationChange::Clear => settings.weather.place = None,
            form::LocationChange::Set(place) => settings.weather.place = Some(place),
            form::LocationChange::Lookup(query) => {
                let found = hub.search_places(&query).await?;
                let place =
                    found.into_iter().next().ok_or_else(|| format!("couldn't find a place called {query:?}"))?;
                settings.weather.place = Some(place);
            }
        }
        tz::adopt_place_zone(&mut settings);
        hub.apply_settings(settings)
    }
    .await;
    match result {
        Ok(_) => (StatusCode::SEE_OTHER, [(LOCATION, "/admin?saved")]).into_response(),
        Err(e) => html(StatusCode::BAD_REQUEST, admin_page(&state, Notice::Error(e), peer, &headers)),
    }
}

/// Shared checks for the admin forms that act immediately.
fn admin_form(state: &AppState, peer: SocketAddr, headers: &HeaderMap) -> Option<Response> {
    if !auth::same_origin(headers) {
        return Some(html(StatusCode::FORBIDDEN, "cross-site form post refused".into()));
    }
    deny(state.auth.check(peer.ip(), headers))
}

fn field(body: &[u8], key: &str) -> String {
    pairs(body).into_iter().find(|(k, _)| k == key).map(|(_, v)| v.trim().to_owned()).unwrap_or_default()
}

async fn find_fantasy(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if let Some(denied) = admin_form(&state, peer, &headers) {
        return denied;
    }
    match state.hub.find_fantasy(&field(&body, "username")).await {
        Ok(search) => html(StatusCode::OK, admin_page_with(&state, Notice::None, peer, &headers, Some(&search))),
        Err(e) => html(StatusCode::BAD_REQUEST, admin_page(&state, Notice::Error(e), peer, &headers)),
    }
}

async fn add_fantasy(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if let Some(denied) = admin_form(&state, peer, &headers) {
        return denied;
    }
    let roster: u32 = field(&body, "roster_id").parse().unwrap_or(0);
    match state.hub.add_fantasy(&field(&body, "league_id"), &field(&body, "league"), roster).await {
        Ok(()) => (StatusCode::SEE_OTHER, [(LOCATION, "/admin?saved#fantasy")]).into_response(),
        Err(e) => html(StatusCode::BAD_REQUEST, admin_page(&state, Notice::Error(e), peer, &headers)),
    }
}

async fn test_takeover(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if let Some(denied) = admin_form(&state, peer, &headers) {
        return denied;
    }
    let Some(kind) = marqueet_core::test_alerts::TestKind::from_id(&field(&body, "kind")) else {
        let notice = Notice::Error("Pick a takeover to test.".into());
        return html(StatusCode::BAD_REQUEST, admin_page(&state, notice, peer, &headers));
    };
    let team = Some(field(&body, "team")).filter(|t| !t.is_empty()).map(marqueet_core::sports::TeamId);
    match state.hub.test_takeover(kind, team.as_ref()) {
        Ok(()) => (StatusCode::SEE_OTHER, [(LOCATION, "/admin?tested#test-takeover")]).into_response(),
        Err(e) => html(StatusCode::BAD_REQUEST, admin_page(&state, Notice::Error(e), peer, &headers)),
    }
}

async fn remove_fantasy(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if let Some(denied) = admin_form(&state, peer, &headers) {
        return denied;
    }
    let roster: u32 = field(&body, "roster_id").parse().unwrap_or(0);
    match state.hub.remove_fantasy(&field(&body, "league_id"), roster) {
        Ok(()) => (StatusCode::SEE_OTHER, [(LOCATION, "/admin?saved#fantasy")]).into_response(),
        Err(e) => html(StatusCode::BAD_REQUEST, admin_page(&state, Notice::Error(e), peer, &headers)),
    }
}

async fn create_feed(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if let Some(denied) = admin_form(&state, peer, &headers) {
        return denied;
    }
    let name = field(&body, "name");
    match state.hub.create_feed(&name) {
        // Shown on this page only: only its hash is kept.
        Ok(token) => html(StatusCode::OK, admin_page(&state, Notice::NewToken { feed: name, token }, peer, &headers)),
        Err(e) => html(StatusCode::BAD_REQUEST, admin_page(&state, Notice::Error(e), peer, &headers)),
    }
}

async fn new_feed_token(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if let Some(denied) = admin_form(&state, peer, &headers) {
        return denied;
    }
    let name = field(&body, "name");
    match state.hub.new_feed_token(&name) {
        Ok(token) => html(StatusCode::OK, admin_page(&state, Notice::NewToken { feed: name, token }, peer, &headers)),
        Err(e) => html(StatusCode::BAD_REQUEST, admin_page(&state, Notice::Error(e), peer, &headers)),
    }
}

async fn change_password(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if let Some(denied) = admin_form(&state, peer, &headers) {
        return denied;
    }
    let (current, password, confirm) = (field(&body, "current"), field(&body, "password"), field(&body, "confirm"));
    let error = |status, message: &str| html(status, admin_page(&state, Notice::Error(message.into()), peer, &headers));
    if password != confirm {
        return error(StatusCode::BAD_REQUEST, "the two new passwords don't match");
    }
    let permit = match state.auth.begin_attempt() {
        Ok(p) => p,
        Err(wait) => return error(StatusCode::TOO_MANY_REQUESTS, &too_many(wait)),
    };
    let auth = Arc::clone(&state.auth);
    let result = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        auth.change_password(&current, &password)
    })
    .await
    .unwrap_or_else(|e| Err(ChangeError::Internal(e.to_string())));
    match result {
        Ok(token) => with_session(redirect("/admin?password#password"), &token),
        Err(ChangeError::WrongPassword) => error(StatusCode::UNAUTHORIZED, "the current password is wrong"),
        Err(ChangeError::Configured) => {
            error(StatusCode::BAD_REQUEST, "this password comes from the device's configuration")
        }
        Err(ChangeError::Invalid(e)) => error(StatusCode::BAD_REQUEST, &e),
        Err(ChangeError::Internal(e)) => {
            log::error!("changing the password failed: {e}");
            error(StatusCode::INTERNAL_SERVER_ERROR, "something went wrong saving the password")
        }
    }
}

async fn revoke_feed(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if let Some(denied) = admin_form(&state, peer, &headers) {
        return denied;
    }
    match state.hub.revoke_feed(&field(&body, "name")) {
        Ok(()) => (StatusCode::SEE_OTHER, [(LOCATION, "/admin?saved#feeds")]).into_response(),
        Err(e) => html(StatusCode::BAD_REQUEST, admin_page(&state, Notice::Error(e), peer, &headers)),
    }
}

async fn login_page(State(state): State<AppState>, ConnectInfo(peer): ConnectInfo<SocketAddr>) -> Response {
    if auth::is_local(peer.ip()) {
        return redirect("/admin");
    }
    if state.auth.setup_pending() {
        return redirect("/setup");
    }
    html(StatusCode::OK, page::login(false))
}

fn with_session(mut res: Response, token: &str) -> Response {
    if let Ok(v) = HeaderValue::from_str(&auth::session_cookie(token)) {
        res.headers_mut().insert(SET_COOKIE, v);
    }
    res
}

async fn login(State(state): State<AppState>, headers: HeaderMap, body: Bytes) -> Response {
    if !auth::same_origin(&headers) {
        return html(StatusCode::FORBIDDEN, "cross-site form post refused".into());
    }
    if state.auth.setup_pending() {
        return redirect("/setup");
    }
    let permit = match state.auth.begin_attempt() {
        Ok(p) => p,
        Err(wait) => return html(StatusCode::TOO_MANY_REQUESTS, page::login_message(&too_many(wait))),
    };
    let attempt = field(&body, "password");
    // Hashing is slow on purpose; keep it off the async executor.
    let auth = Arc::clone(&state.auth);
    let result = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        auth.login(&attempt)
    })
    .await;
    match result.ok().flatten() {
        Some(token) => with_session(redirect("/admin"), &token),
        None => html(StatusCode::UNAUTHORIZED, page::login(true)),
    }
}

/// "Too many tries" with how long to wait, rounded up to a whole second.
fn too_many(wait: std::time::Duration) -> String {
    let secs = wait.as_secs() + u64::from(wait.subsec_nanos() > 0);
    format!("Too many tries. Wait {secs} second{} and try again.", if secs == 1 { "" } else { "s" })
}

async fn setup_page(State(state): State<AppState>) -> Response {
    if !state.auth.setup_pending() {
        return redirect("/admin");
    }
    html(StatusCode::OK, page::setup(None))
}

async fn setup(State(state): State<AppState>, headers: HeaderMap, body: Bytes) -> Response {
    if !auth::same_origin(&headers) {
        return html(StatusCode::FORBIDDEN, "cross-site form post refused".into());
    }
    if !state.auth.setup_pending() {
        return redirect("/admin");
    }
    let (code, password, confirm) = (field(&body, "code"), field(&body, "password"), field(&body, "confirm"));
    if password != confirm {
        return html(StatusCode::BAD_REQUEST, page::setup(Some("The two passwords don't match.")));
    }
    let permit = match state.auth.begin_attempt() {
        Ok(p) => p,
        Err(wait) => return html(StatusCode::TOO_MANY_REQUESTS, page::setup(Some(&too_many(wait)))),
    };
    let auth = Arc::clone(&state.auth);
    let result = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        auth.finish_setup(&code, &password)
    })
    .await
    .unwrap_or_else(|e| Err(SetupError::Internal(e.to_string())));
    match result {
        // Straight into the friendly welcome steps.
        Ok(token) => with_session(redirect("/welcome"), &token),
        Err(SetupError::Done) => redirect("/admin"),
        Err(SetupError::WrongCode) => {
            html(StatusCode::UNAUTHORIZED, page::setup(Some("That code isn't the one on the screen.")))
        }
        Err(SetupError::Invalid(e)) => html(StatusCode::BAD_REQUEST, page::setup(Some(&e))),
        Err(SetupError::Internal(e)) => {
            log::error!("setup failed: {e}");
            html(StatusCode::INTERNAL_SERVER_ERROR, page::setup(Some("Something went wrong saving the password.")))
        }
    }
}

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if !auth::same_origin(&headers) {
        return html(StatusCode::FORBIDDEN, "cross-site form post refused".into());
    }
    state.auth.logout(&headers);
    let mut res = redirect("/login");
    if let Ok(v) = HeaderValue::from_str(&auth::clear_cookie()) {
        res.headers_mut().insert(SET_COOKIE, v);
    }
    res
}
