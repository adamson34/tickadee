//! Marqueet server: polls data providers on an adaptive schedule, keeps the
//! last good data when an upstream fails, and pushes ready-to-render ticker
//! content, widgets, alerts and display settings to the display over a local
//! WebSocket.
//!
//! - [`schedule`]: pure polling policy (fast when live, slow when idle, backoff).
//! - [`store`]: pure per-league cache with failure tracking and staleness.
//! - [`content`]: pure games -> ticker/crawl segments and widget views.
//! - [`settings_store`]: settings persisted in SQLite.
//! - [`hub`]: shared state, settings, pollers.
//! - [`tz`]: the configured time zone.
//! - [`web`]: `/ws`, `/api/games`, `/api/alerts`, `/api/settings`, `/healthz`.
//! - [`admin`]: the admin page and who may use it.
//! - [`feed_api`]: content pushed by local programs.

pub mod admin;
pub mod content;
pub mod device;
pub mod feed_api;
pub mod hosts;
pub mod hub;
pub mod schedule;
pub mod settings_store;
pub mod store;
pub mod team_art;
pub mod tv_output;
pub mod tz;
pub mod web;

use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;

use marqueet_core::settings::Settings;
use tokio::net::TcpListener;

pub use hub::{FantasySearch, Hub, Providers};
pub use schedule::Policy;
pub use settings_store::SettingsStore;

/// Admin access: a given password (else first-boot setup) and where the
/// setup page can be reached, for the first-boot screen.
#[derive(Clone, Debug, Default)]
pub struct AdminOptions {
    pub password: Option<String>,
    pub setup_urls: Vec<String>,
    /// The device's host name, which requests may use besides its IPs and
    /// `localhost` (see [`hosts`]).
    pub hostname: Option<String>,
    /// More names requests may use (a reverse proxy's).
    pub extra_hosts: Vec<String>,
}

/// Starts polling for the leagues in `settings` and serves until `shutdown`
/// resolves. With `db`, settings (and the admin password made at first boot)
/// are saved.
pub async fn run(
    listener: TcpListener,
    providers: Providers,
    settings: Settings,
    policy: Policy,
    db: Option<SettingsStore>,
    admin: AdminOptions,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    let auth = Arc::new(admin::auth::Auth::new(admin.password, db.clone(), admin.setup_urls));
    let hub = Hub::new(settings, policy, db);
    hub.start(providers);
    let hosts = hosts::AllowedHosts::new(admin.hostname.as_deref(), &admin.extra_hosts);
    let app = web::router(Arc::clone(&hub), auth, hosts).into_make_service_with_connect_info::<SocketAddr>();
    let result = axum::serve(listener, app).with_graceful_shutdown(shutdown).await;
    hub.stop();
    result
}
