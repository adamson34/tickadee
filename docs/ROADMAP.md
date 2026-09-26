# Roadmap

Marqueet is built in phases. Each phase ends with a review before the next
one starts; work lands on `dev` through feature-branch PRs (several per
phase). `main` is only updated for releases, starting with v1
([ADR-0010](adr/0010-main-is-for-releases.md)). Architectural decisions are recorded in
[`adr/`](adr/).

## Phase 1: repo + LED display on mock data ✅

- [x] Cargo workspace, rustfmt, clippy (`-D warnings`), CI (incl. ARM64), cargo-deny, Dependabot
- [x] Normalized game schema covering all five sports, plus mock fixtures
- [x] Generic ticker segments, alerts, display config, layout math ([ADR-0003](adr/0003-generic-segments-and-alerts.md))
- [x] Hand-drawn LED font plus a Scale2x large font
- [x] LED renderer: dot grid, glow, flicker, stepped or smooth scroll ([ADR-0004](adr/0004-led-rendering-pipeline.md))
- [x] Two-line game blocks, league headers, team colors made LED-safe
- [x] Score flash (invert blink, then a fading boost)
- [x] Crawl with upcoming games; placeholder LED clock in the widget area
- [x] Layouts for 1080p, 1366x768 and 4:3; headless `--screenshot` / `--record`
- [x] Parakeet dot-grid logo, generated SVGs, LED welcome screen ([ADR-0005](adr/0005-dot-grid-brand-source-of-truth.md))
- [x] Concept mockup GIF of the planned design ([ADR-0007](adr/0007-led-ticker-flat-widgets.md))
- [x] Headless `--score` to script a scoring alert
- [x] Measured on a real Raspberry Pi 4 (2026-09-24): about 22 to 28 fps at 1080p, about 9 at 4K (the Pi image's 4K TVs get 1080p)
- [ ] 60 fps on a Pi 4 (the LED glow passes are the cost), and a release check on real hardware
- [x] Render at 1080p on 4K screens automatically (scaled up by the GPU), on any install; a Resolution and frame-rate choice on the admin page
- [x] Switch the TV's own output mode to match (a small helper outside the snap, `packaging/tv-output/`, set up by the installer and the Pi image)

## Phase 2: live data ✅

- [x] `DataProvider` trait in core ([ADR-0009](adr/0009-espn-provider.md))
- [x] ESPN provider: fetch plus a pure `normalize()`, tested against saved JSON fixtures (real captures + labeled synthetic live states)
- [x] `server` crate (axum + tokio): `/ws` display feed, `/api/games` JSON, `/healthz`
- [x] Poll scheduler: 12 s while live, 60 s near kickoff, 5 min on game days, 20 min idle; ±10% jitter; exponential backoff to 5 min; last good data kept and marked stale (DELAYED) after 3 failures
- [x] WebSocket protocol (`core::protocol`)
- [x] The server formats games into segments so the display stays source-agnostic
- [x] Display client with reconnect (keeps last scores while disconnected); the mock feed becomes `--mock`

## Phase 3: events + takeovers ✅

- [x] Event engine (`core::events`): diff snapshots into touchdown / field goal / safety / extra point / two-point / home run / grand slam / runs / goal / final / score correction; play text breaks ties, score delta otherwise; basketball only alerts on finals
- [x] Deterministic alert ids (`<game>:<kind>:<away>-<home>`); no alerts on the first snapshot or on stale data
- [x] Mock feed runs the real engine
- [x] Takeover in the widget area: drifting team-color stripes and dot texture, LED-block kicker / headline / play / score box / "your player" pill, fade in and out, 10 s each, queued one at a time (stale ones dropped)
- [x] ~~Play text and "your player" pill in vector text~~: superseded; takeovers keep their LED text ([ADR-0012](adr/0012-display-themes.md)), and the play text shows in vector text in the spotlight's LAST PLAY strip
- [x] Tests for event detection in every sport
- [x] Server runs the engine on every poll (only against a fresh previous snapshot), dedupes by id, pushes alerts to displays after the content update; `GET /api/alerts`

## Phase 4: widgets + admin

- [x] UI renderer: CPU canvas (rounded rects, vector text via swash with bundled Barlow Condensed (OFL), LED-block text) uploaded only when it changes, composited by the GPU with fades ([ADR-0007](adr/0007-led-ticker-flat-widgets.md))
- [x] Header bar: LIVE badge (grey when nothing is live), league filter label, LED clock
- [x] Crawl restyled as flat condensed text with a TONIGHT / TODAY / UP NEXT tag; drawn once per change and scrolled on the GPU (tiled strip, no per-frame uploads)
- [x] Feed API ([docs/FEEDS.md](FEEDS.md)): local programs in any language push ticker segments, crawl lines and flashes/takeovers over HTTP (`POST /api/feeds/<name>`, `/alert`, `DELETE`), with a per-feed Bearer token created on the admin page (stored apart from settings), expiring content, size limits and alert rate limits. The extension point instead of compiled-in plugins
- [x] Widget view models built on the server (`core::widgets`), sent with each content update; the display only draws them
- [x] Widgets: game of the day (picks a favorite's live game, else the closest live game, else next up, else latest final; big team names, LED-block scores, situation chips, line score) and scores list (live, then finals, then upcoming)
- [x] Standings in the ticker: each favorite's place rides on its league header (`NFL  LAB 1ST / WEST 3-0`), or gets its own small segment on days the league has no games
- [x] Standings widget: ESPN standings (divisions for NFL/MLB/NBA/NHL, conferences for WNBA/MLS, the table for soccer; not college), refreshed every 30 min; shows a favorite's group (scrolled so they're visible), else the featured game's; sport-specific columns
- [x] Weather widget: Open-Meteo (free, no key, CC BY 4.0) via `marqueet-provider-openmeteo`; location by city search or "lat, lon" on the admin page, °F/°C; current conditions plus 5 days with drawn icons; fetched every 15 min while the weather widget or the ticker's weather is on (see SECURITY.md for where the location goes). The header clock replaces the clock widget
- [x] Weather in the ticker (ticker first: every source gets a ticker presence): an LED segment leading each loop with a multi-color LED icon (`Part::Icon`, icons as data in `assets/icons.txt`), temperature, city, high/low, and a RAIN/SNOW/STORMS heads-up when the chance is 50%+ in the next few days; on by default once a location is set, with its own admin toggle
- [x] Severe weather alerts from the US National Weather Service (`marqueet-provider-nws`; free, no key, public domain), checked every 2 minutes whenever a location is set: in effect → a colored segment at the very front of the ticker (red warning, orange watch, amber advisory, with an LED icon); new severe/extreme warnings take over the screen, watches flash; updates don't re-alert; outside the US the check stops for that place. Admin toggle, on by default
- [x] Admin web page (`/admin`): server-rendered HTML with an escape helper (no template engine), plain CSS, one small hand-written script for drag-to-reorder; works with scripting off and on phones. Open on the device; from the network after logging in (session cookie, HttpOnly, SameSite=Strict), or first-boot setup when no password exists yet; cross-site posts refused, strict CSP; requests must name the device (IP, `localhost` or its own name) so DNS-rebinding pages are refused
- [x] Layout editor: five presets (wide left, wide right, halves, three, single) as `widget_layout` in the display settings; the admin page shows layout previews and slot boxes that follow the choice with CSS alone, a dropdown per slot (phones, no JS), and drag-a-slot-onto-another to swap on desktop. Widgets scale down to fit narrow slots
- [x] SQLite settings store (`--db`), applied live: leagues and order (pollers start/stop), favorite teams, takeover policy (all / favorites / off), widget slots, display look
- [x] Overnight quiet hours (screen blanks); now night mode, dim or black
- [x] Stale data marked DELAYED on the league header (since Phase 2)
- [x] Time zone setting (IANA name from the system zoneinfo via jiff, with daylight saving; blank = the device's zone): drives start times, quiet hours, and the display's clock (sent as a UTC offset with the display settings)

## Phase 5: fantasy

- [x] Sleeper provider (`marqueet-provider-sleeper`): username / league / team lookup, this week's matchup with starters' live points and ESPN ids, daily player-list cache
- [x] Fantasy setup on the admin page (find by Sleeper username, pick league and team, up to 4 teams); matchups polled every 30 s while NFL games are live, else every 10 min
- [x] Fantasy on the ticker (each followed matchup near the front: teams stacked, the leader's score bright, a flash when the lead changes) and a matchup widget (LED totals, starters side by side by lineup slot)
- [x] Player matching through Sleeper's `espn_id`; fantasy details in takeovers ("YOUR STARTER | J. ALLEN 24.1 PTS", or the opponent's); with "favorites only" takeovers, a play by your starter counts as a favorite's
- [x] Per-widget settings (moved from Phase 4): each slot is `{kind, option}` (a league for Game of the Day / Scores / Standings, a followed team for Fantasy); `WidgetKind` carries its id, label and choices, so the admin page, the form parser and `build_views` share one table. No dynamic trait objects or `schemars`: widgets are built in, and outside code extends the sign through the feed API. Old settings (bare kinds) still load

## Phase 6: kiosk

- [x] Packaging decided ([ADR-0011](adr/0011-packaging-snap.md)): one strictly confined snap with `server` and `display` daemons next to `ubuntu-frame` (gpu-2404 graphics, waits for Frame's Wayland socket), `snap set marqueet reset-password=true`; built in CI for amd64 and arm64. systemd units for from-source installs
- [x] mDNS `marqueet.local`: the installer names the computer and installs Avahi (the Pi image is Ubuntu Server, so this covers it; ADR-0014)
- [x] First-boot setup (server): with no admin password, a one-time 6-digit code is sent only to displays on the device and logged; guesses are throttled server-wide (one check at a time, a growing wait after failures up to a minute), and wrong guesses never change the code; `/setup` takes the code plus a new password; remote visitors get only the setup page
- [x] First-boot screen (display): replaces the widget area while setup is pending: a QR code for the setup page (`qrcodegen`), the `.local` and IP addresses, and the code in LED digits; the ticker keeps running above
- [x] Password creation at setup (PBKDF2-SHA256, 600,000 iterations, via `ring`), stored in the database; the code then expires and you're logged in
- [x] SSH is an OS setting, off by default in the image, not an admin toggle (a confined app that could enable SSH is a target; ADR-0011)
- [ ] Optional self-signed HTTPS (moved to the hardening pass)
- [x] Password reset through a file (`--reset-file`, e.g. on the boot partition): back to first-boot setup, acted on once even if the file can't be deleted
- [ ] Optional: WiFi captive portal when there's no Ethernet (now planned under [Setup and control](#setup-and-control))

## Phase 7: distribution

- [x] Edge builds: every merge to `dev` publishes both snaps and checksums as the rolling `edge` pre-release
- [x] One-command installer (`install.sh`) for Ubuntu 24.04 on x86 and Raspberry Pi: Frame, Mesa, Avahi, the `marqueet` host name, the snap (with a SHA-256 download check; not a signature until the Snap Store, ADR-0014), connections, boot-to-ticker (asks before turning off a desktop); tested in CI on a fresh machine
- [x] Flashable Raspberry Pi image: Ubuntu Server 24.04 for Pi (Ubuntu's signed checksums verified) plus cloud-init that names it `marqueet`, turns SSH off and runs the installer on first boot (retrying until online); daily update check; `reset-password` file on the SD card; plain-language README on the card; built and checked in CI, published with each edge build
- [x] Developer switch on the Pi image: a public key on the SD card turns on key-only SSH at boot, a `marqueet-ssh-off` file turns it off ([PI-DEVELOPMENT.md](PI-DEVELOPMENT.md)); tested in CI with a fake system
- [x] Edge builds publish only after the full CI checks pass on the same commit; the `gpu-2404` part is pinned to a commit ([ADR-0014](adr/0014-edge-releases-and-server-image.md))
- [x] Snap Store: `marqueet` registered; every CI-passing merge to `dev` uploads to the store's `edge` channel, and the installer uses the store when it has a build (store-signed, snapd refreshes and can revert), moving GitHub-installed copies over with `--amend`
- [x] Release workflow with checksums: a `v*` tag on `main` publishes the store's `stable` channel and a GitHub release (snaps, Pi image, SHA256SUMS, notes from the changelog); v1.0.0 is the first
- [ ] Snap Store auto-connection approval for `wayland` and `gpu-2404` (until then the installer and Pi image make the connections)

## Themes

- [x] Three styles for the crawl and widgets ([ADR-0012](adr/0012-display-themes.md)): Broadcast (default: TV score graphics in the teams' colors), Ballpark (painted scoreboard with number plates), Varsity (jersey-number scores); each with a preset palette of nine colors by role and a team-colors switch. `--theme` for mock screenshots
- [x] Pick a look in the first-time welcome steps too (fifth step, skippable)
- [x] Header bar and score takeovers in each look's colors, lettering and background pattern
- [x] Pick a look on the admin page, with sketches drawn by the server in each look's colors (SVG, so the strict CSP holds)
- [x] Opt-in logos from the scores provider (ESPN): off by default, only for today's teams, only from ESPN's image CDN, cached; your own logos win (ADR-0013 amendment)
- [x] Your own team colors and logos ([ADR-0013](adr/0013-bring-your-own-team-art.md), [TEAM_PACKS.md](TEAM_PACKS.md)): none ship; people upload a PNG and pick colors per team, or import a team pack file; logos on the LED ticker and in every theme's widgets, colors everywhere including takeovers
- [x] Build your own: change any of the nine colors, go back to the look's own, a warning for hard-to-read pairs, and a short theme code to share a look or paste someone else's

## Spotlight

- [x] One game fills the widget area: automatically when it's the only one live, or a game picked on the admin page; the look's game view plus a last-play strip
- [x] Primetime: a football game alone in its league (Thursday night) gets the spotlight with other sports on; on by default, with a checkbox
- [x] A favorite team's live game gets the spotlight even with other games on (the closer one if two play); on by default, with a checkbox
- [x] Scoring summary, team stats and leaders from the provider's game summary (only for the spotlighted game)
- [x] The ticker tells the spotlighted game's story: win chance, key stats and the latest scoring plays after its score (tonight's other games are in the crawl, since each game shows in one band)

## Playoffs

The postseason is when a ticker matters most. MLB first (October), then the
NBA, NHL and NFL brackets on the same model.

- [x] Playoff bracket widget: the league's bracket by round (for MLB: Wild Card, Division Series, Championship Series, World Series), each matchup with seeds, the series score ("leads 2-1", "series tied 1-1") and the next game's day, time and TV; your teams highlighted, eliminated teams dimmed; finished rounds collapse so the current one has room
- [x] Series status everywhere: the ticker and crawl show a playoff game's series ("GM 4 · LEADS 2-1", with the leading team), and the spotlight shows it next to the score
- [ ] Series takeovers: a clinch ("ADVANCES", "WINS THE SERIES", "WORLD SERIES CHAMPIONS") gets its own flash and takeover, favorites first
- [ ] Bracket in the crawl on off days: the round's matchups and series scores when no playoff game is live
- [ ] The bracket takes the spotlight's place automatically during the playoffs when no game is live (a checkbox), and the admin page can pin it
- [x] Data from the provider's postseason data (ESPN's series info on each game; ESPN has no bracket feed, so the server keeps every playoff game it sees and fetches the postseason's earlier days once), normalized in core so another provider can fill it; the 2025 MLB postseason is a test fixture

## Custom takeovers

Fans make their team's takeover their own; nothing trademarked ships with
Marqueet (the same rule as team art, ADR-0013). Set per team and per play
with the team's colors and logo, carried in team packs, previewed with the
test buttons.

- [x] Words: the team's own headline ("KINGDOM TD!" for "TOUCHDOWN") and a second line, for touchdowns, home runs, grand slams and goals
- [x] Art: a team's own LED picture or animation (a PNG, a PNG sprite strip or an animated GIF, one light per pixel, up to 96x48 and 48 frames), above the words or on its own first; `--takeover-art FILE` previews it in the demo
- [ ] Art: a simple pixel editor on the admin page
- [ ] Look: background pattern, colors, how long it stays up, how much it flashes
- [ ] Sound: a team's own horn or chant, once the goal horn exists

## Game day

- [x] Football drive tracker in the spotlight (taking turns with the stats): the drive so far, down and distance, a field with the start, ball and first-down line, the latest plays; the play tracker (at-bat and drive) is a setting, on by default
- [x] Baseball at-bat panel in the spotlight: pitcher and batter with their lines, the count, outs and runners, a strike zone with this at-bat's pitches (numbered, colored by call) and the pitches in words (type and speed); refreshed with each poll while the game is live. No player photos
- [x] Betting lines as information: the spread and over/under with upcoming games in the crawl and in the spotlight, off by default (no sportsbook names, links or accounts)
- [ ] Your fantasy players in the ticker: when a player on your Sleeper roster scores, a flash with their name and points (and the opponent's players, dimmed); a "your players" line in the spotlight
- [ ] Goal horn / touchdown sound: an optional sound through the TV when a favorite team scores (a volume setting, per-sport sounds, silent during night mode); sounds bundled under a free license
- [ ] Split-screen spotlight ("RedZone"): two close live games side by side instead of one
- [ ] Pregame countdown: a few hours before a favorite team plays, a countdown with the channel and venue, then the spotlight at kickoff
- [ ] Rankings and races: the college top 25 in the crawl, and late-season standings races (wild card, magic numbers)

## Setup and control

For the fans who'll set this up: flash, plug in, scan, with no keyboard and
no terminal.

- [ ] WiFi setup without a keyboard: with no network, the Pi opens its own WiFi network with a captive page; you join from your phone and pick your home WiFi (needs network-manager control from the snap)
- [ ] An Imager catalog entry (os-list JSON), so Raspberry Pi Imager offers its WiFi and user settings for the Marqueet image
- [ ] A phone remote: a small page with big buttons (spotlight this game, pause takeovers for an hour, night mode now, next widget)
- [ ] Messages from the admin page: type a message and how long it shows ("Happy birthday Sam!", today only) without writing a feed script
- [ ] A friendlier first-boot screen: the parakeet, the device's address and progress, without the system messages
- [ ] Settings backup and restore: one file with leagues, teams, look, widgets, feeds and team art, to move to a new device

## Beyond sports

- [ ] News headlines: the provider's news for your teams (injuries, trades) in the crawl, plus any RSS or Atom feed
- [ ] Stocks and crypto in the ticker (a free, keyless quote source; your symbols on the admin page)

## Phase 8: Home Assistant

Starts with an ADR: the browser renderer, authentication behind the add-on's
proxy (every request comes from the proxy's address, so "on the device" can't
mean loopback there), and refusing a display on a different protocol version.

For people who already use Home Assistant as their home dashboard: the ticker
sits at the top of their dashboard, with no second device or OS needed.

- [ ] Embeddable ticker page (`/embed/ticker`): just the LED ticker, sized for a card, fed by the same `/ws` feed (scores, weather, fantasy, flashes, feed items)
- [ ] Browser renderer: compile the existing wgpu display to WebAssembly (WebGL/WebGPU), so the embed looks the same as the device; still Rust-only, no npm
- [ ] Docs: add it with Home Assistant's built-in Webpage card (nothing to install in Home Assistant)
- [ ] Home Assistant add-on: the Marqueet server as an add-on container (amd64 + arm64) running inside Home Assistant OS, admin page through ingress; for people without a Marqueet device
- [ ] Optional integration: sensors (live games, fantasy score) and a "show message" service built on the feed API

The device display stays native (ADR-0002); the embed is an extra view for
dashboards, not a replacement.

## Later

- [ ] Behavior before the clock syncs (a Pi has no battery clock): show SETTING CLOCK and hold night mode, start times and feed expiry until it's right ([ADR-0015](adr/0015-security-review-decisions.md))
- [ ] Keep the last scores on disk, so an offline start isn't empty
- [ ] HTTPS for the admin page (plain HTTP is accepted for now, ADR-0015)
- [ ] Show "SSH is on" on the screen while the Pi image's developer switch is on

- [ ] Non-sports sources beyond news and stocks (see [Beyond sports](#beyond-sports))
- [ ] More sports providers as fallbacks for ESPN

## Non-goals

- A browser-based display, or any JavaScript toolchain ([ADR-0001](adr/0001-rust-everywhere-no-js-toolchain.md))
- Physical RGB LED matrix panels
- Cloud accounts or telemetry
- Wagering or sportsbook integration: no bet tracking, sportsbook links or accounts (betting lines are shown only as information, off by default)
