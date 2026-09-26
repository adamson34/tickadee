# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Changed

- Security docs: SECURITY.md now says what's readable on your network
  without logging in, that the admin page is plain HTTP, that the SD card
  gives full control, and what leaves the device for each provider.
  [ADR-0015](docs/adr/0015-security-review-decisions.md) records those
  decisions, and [docs/SECURITY-REVIEW.md](docs/SECURITY-REVIEW.md) lists
  every finding from the security review and what became of it.
- README, CLAUDE.md, ROADMAP and the ADR index brought up to date with v1
  (status, layout, the Pi 4 frame rate as measured, examples without real
  team names).

### Added

- **Screen** settings on the admin page (Display): a **Resolution** to
  draw at (Automatic, 4K, 1440p, 1080p, 720p or the screen's own) and a
  frame rate (60, or 30 to keep a Pi cooler). Automatic draws at 1080p on
  a 4K TV and scales it up, so a Pi 4 on a 4K TV is smooth out of the box
  (it managed about 5 frames a second drawing 4K).
- The TV's own output mode follows that setting, where the installer set
  Marqueet up with Ubuntu Frame (the Pi image, the one-command installer):
  a small helper outside the snap (`packaging/tv-output/`) picks the closest
  mode the TV offers, for example 1080p at 60 Hz instead of 4K at 30, and
  the admin page shows what the TV is getting and what it offers.
- LED art can go **in the background**: big and dimmed behind the words
  for the whole takeover (kept out of the score box so it reads). The
  preview option is now `--art-placement above|intro|behind`.
- A football drive tracker in the spotlight, taking turns with the stats:
  who has the ball and the drive so far ("7 PLAYS · 48 YDS · 3:21"), down
  and distance (red in the red zone), a field with the drive's start, the
  ball and the first-down line in the teams' colors, and the latest plays
  with their yards. A live football spotlight now updates with every poll,
  like baseball's at-bat.
- **The live play tracker** is now a setting (Spotlight, on by default):
  turn it off to keep the spotlight to the game's stats.
- LED art for your team's takeovers: in **Your team colors and logos**,
  upload a picture or an animation (a PNG, a PNG strip of frames, or an
  animated GIF; one light per pixel, up to 96x48 and 48 frames) and choose
  whether it shows above the words or on its own first. It plays when that
  team scores, in test takeovers, and travels in team packs
  (`takeover_art`). Preview a file without a server:
  `marqueet-display --mock --takeover-art art.gif --score mock:nfl:1:home:6`.
- Betting lines, as information: turn on **Show betting lines** (Display)
  to see the spread and over/under ("BUF -7 · O/U 50.5") with upcoming games
  in the crawl and in the spotlight. Off by default; from ESPN's scoreboard,
  without sportsbook names or links. The roadmap's "no betting features"
  non-goal is now "no wagering or sportsbook integration".
- A baseball at-bat panel in the spotlight, like a broadcast's pitch
  tracker: the pitcher's and batter's lines for the game, balls, strikes and
  outs, the runners, a strike zone with this at-bat's pitches numbered and
  colored (balls green, strikes red, in play blue), and each pitch in words
  ("STRIKE SWINGING · CUTTER 83"). It shows while an at-bat is on and updates
  with every poll (about every 12 seconds); between innings the game's stats
  take turns as before.
- A **Ticker size** slider on the admin page (Display): make the LED ticker
  and crawl bigger (up to 60% of the screen) or a bit smaller; the widgets
  get the rest.
- Your team's own takeover words: in **Your team colors and logos**, set a
  headline ("KINGDOM TD!" instead of "TOUCHDOWN") and a second line for a
  team's touchdowns, home runs, grand slams and goals. They're used when
  that team scores, in test takeovers, and travel in team packs
  (`takeovers`).
- **Test a takeover** on the admin page: buttons for a touchdown, home
  run, grand slam, hockey goal, soccer goal, weather warning and a feed
  message play that takeover on the screen now. Each replays the last real
  one of its kind, or makes one from today's games; pick a team to see it
  score in its colors.
- A playoff bracket widget (**Playoff bracket** on the admin page, for a
  league or automatically the one in its playoffs): a column per round
  (for MLB: wild card, division series, LCS, World Series), each series with
  both teams' wins, lines to the series that fed it, the next game's day and
  time or LIVE, winners in bold, knocked-out teams dimmed and your teams
  highlighted. ESPN has no bracket feed, so the server keeps every playoff
  game it sees and, when a postseason is on, fetches its earlier days once.
- Playoff games show their series: "ALCS G4 / SEA 2-1" in the ticker, the
  round in the crawl, and "ALCS GM 4 · SEA LEADS 2-1" in the spotlight.
- A watchdog for the display: if its loop stops turning for 30 seconds (a
  freeze like the one fixed in 1.0.0), it exits and snapd (or systemd)
  restarts it three seconds later, instead of leaving a frozen picture.
- The ticker tells the spotlighted game's story: right after its score come
  the win chance (while it's live), three key team stats and the three latest
  scoring plays, newest first. Tonight's other games are in the crawl.

### Security

- Admin sessions now end after a week unused or 30 days after logging in
  (the cookie carries the same `Max-Age`), and all of them end when the
  password changes.
- **Change the admin password** on the admin page (current password, new one
  twice), under the same guessing limits as logging in. A password set with
  `MARQUEET_ADMIN_PASSWORD` is changed there instead.
- Feed tokens are kept only as a SHA-256 hash and shown once, when made;
  **New token** replaces a lost one. Tokens saved by older versions are
  hashed on the first start.
- A feed's content can be replaced at most once every 2 seconds (429 with
  `Retry-After` otherwise), so a runaway script can't keep the display
  redrawing.
- CI checks the minimum Rust version (1.88), and `cargo deny` bans OpenSSL,
  aws-lc and ttf-parser so the TLS and font decisions can't quietly regress.

### Fixed

- Baseball's last play showed single pitches ("Strike 1 Looking") and new
  batters. Only an at-bat's result is shown now ("Abreu grounded out to
  second."), and it stays up until the next one.
- A real score that lands on the same numbers as one taken back earlier (a
  disallowed goal, then a real one) is announced again; a score reviewed and
  given back within 10 minutes still isn't announced twice.
- The `--reset-file` help and README now say the file is checked when the
  server starts (the snap and the Pi image restart it for you).

## [1.0.0] - 2026-09-25

The first release. Marqueet turns a TV or monitor and a Raspberry Pi, mini PC
or old laptop into a live LED-style sports ticker: scores, schedules, weather
and your fantasy matchup scrolling on top, widgets below, and a flash or a
full-screen takeover when someone scores. Flash the Pi image or run the
one-line installer, scan the QR code, and set it up from your phone.

- Install: the Pi image (`marqueet-pi.img.xz` below) or
  `curl -fsSL https://raw.githubusercontent.com/adamson34/marqueet/main/install.sh | sudo sh`
  on Ubuntu 24.04. Both install from the Snap Store's stable channel and
  update by themselves.
- Everything below has landed since the project started.

### Release

- Releases: a version tag on `main` publishes the snaps to the Snap Store's
  `stable` channel and a GitHub release with the snaps, the Pi image and
  SHA256 checksums, after the full checks pass. `dev` keeps publishing
  `edge`.
- The installer installs `stable` (falling back to `edge` before there's a
  stable build), and keeps whichever channel a device already follows;
  `MARQUEET_CHANNEL=edge` picks edge.

### Fixed

- A mouse pointer showed on the TV. The installer (and so the Pi image) now
  tells Ubuntu Frame not to draw one; on an existing install, run
  `sudo snap set ubuntu-frame config="cursor=null"`.
- The screen froze when the spotlight's last play or a scoring play was
  too long to fit: cutting it short with "…" never finished.
- ESPN play text that started with a space or broke onto a second line
  (before a penalty) is now shown on one line.
- Game times showed in UTC on a fresh Pi (Imager's time zone setting doesn't
  apply to the Marqueet image). Picking your town, in the welcome steps or on
  the admin page, now also sets the time zone from the town when none is
  chosen, so fans never need to know a zone name.
- The first-boot setup card disappeared when the screen's resolution changed
  (the display rebuilt its layers but thought the card was still drawn).
- Setup and every admin form failed in real browsers with "cross-site form
  post refused": under `Referrer-Policy: no-referrer`, browsers send
  `Origin: null` on the page's own form posts. The admin page now uses
  `same-origin`, which still sends nothing to other sites.
- The Pi image's developer SSH switch never ran: it waited for cloud-init's
  final stage, which itself waits for startup to finish, so systemd dropped
  it to break the loop. It now waits for cloud-init's first stage, which
  creates the account.
- On phones, the °F/°C choice in the welcome steps sat above its label
  instead of beside it.
- A two-point conversion that arrives in the poll after its touchdown (+6,
  then +2) is now reported as a two-point conversion instead of a safety.

### Changed

- Each game now shows in one band: the ticker has the scores (live games
  and finals) and the crawl has the schedule. A league with no scores yet
  keeps its next three games in the ticker. Before, upcoming games showed in
  both.
- Slower default speeds: ticker 15 (was 22), crawl 20 (was 30). Saved
  settings keep their speeds.
- **A friendlier team picker** (admin page and welcome steps): your picks
  sit on top as tags (tap one to remove it), one search box finds teams in
  every league at once (or a league by name), leagues start folded with a
  "2 picked" count, and rows are bigger to tap. Still plain checkboxes
  without JavaScript. Script-only controls now also show on the welcome
  pages (they were hidden there).

- **Security: edge builds wait for CI.** The `edge` release (which devices
  install from) is published only after the full checks pass on the same
  commit, and the `gpu-2404` snap part is pinned to a commit. ADR-0014 records
  that the Pi image is Ubuntu Server, how updates work today, and the move to
  the Snap Store for signed updates with rollback.

- **Security: feed posts and team art are bounded.** A feed segment has at
  most 24 parts and gaps of at most 64 columns, feeds can't send logos, and
  the display measures ticker content before drawing it, skipping segments
  that don't fit (so no post can make it allocate without limit). Team art is
  capped at 250 teams; logos go to the display after the scores, in chunks of
  at most 4 MB (protocol version 5); team packs are decoded off the server's
  async threads; logos may be at most 2048 pixels on a side.
- **Security: guessing is throttled.** Setup codes and passwords are checked
  one at a time across the server, with a wait after failures that grows to a
  minute; a flood of logins can no longer pin the CPU. Wrong setup codes no
  longer replace the code on screen (so nobody can keep swapping it on you).
- **Security: requests must name the device.** The server refuses requests
  whose `Host` isn't one of its IPs, `localhost` or its own name (blocking
  DNS rebinding), and `/ws` refuses browser pages from other sites. Use
  `--allowed-host` for a reverse proxy's name.

- The demo data (`--mock`, tests) now uses made-up teams and players
  (Kansas City Kingdom, Buffalo Blizzard, Los Angeles Stars…) instead of real
  ones, and the README says Marqueet isn't affiliated with any league, team,
  player or ESPN. Live data still shows real names, as a scoreboard should.

- The widget area no longer uses dark rounded cards with amber labels; see
  Themes. Widget data now carries team colors and separate scores (protocol
  version 3; update the server and display together).

- The server no longer refuses to listen on the network without
  `MARQUEET_ADMIN_PASSWORD`: without a password it starts in setup mode.
- **The crawl is flat text now** (ADR-0007: LED for the ticker only): league
  in amber, matchup in white, start time and TV muted, behind an amber
  TONIGHT / TODAY / UP NEXT tag. It's drawn once when the schedule changes
  and scrolled by the GPU. `crawl_speed` now means tenths of the crawl's
  height per second (the same pace as before); `crawl_rows` is ignored and
  `--crawl-rows` is gone.
- **Renamed from Tickadee to Marqueet** (marquee + parakeet): "Tickadee" is an
  existing time-clock product ([ADR-0008](docs/adr/0008-name-marqueet.md)).
  Crates are now `marqueet-core` / `marqueet-display`, the mascot is a
  parakeet, and the repository moved to `adamson34/marqueet`.

### Added

- Night mode (was quiet hours) can dim the screen instead of turning it
  black: pick **Dim** or **Black** on the admin page's Time section. Dim is
  the default for newly turned-on night mode; a schedule saved before stays
  black.
- Primetime spotlight (on by default, with a checkbox): a football game
  that's the only one live in its league, like a Thursday night game, gets
  the spotlight even with baseball or other sports on.
- A fantasy matchup stays on screen beside the spotlight (when a fantasy
  widget is set up), so points keep moving with the game. Leader rows with
  no leader on either side yet are left out.
- Game stats in the spotlight: team stats (with the win chance while the game
  is live), leaders and the scoring plays take turns in a panel beside the
  game, a new one every 8 seconds. The strip underneath shows the last score
  when there's no last play. Fetched from ESPN's game summary for the
  spotlighted game only, every 30 seconds while it's on.

- **Your team gets the spotlight.** While one of your favorite teams is
  playing, its game fills the bottom of the screen even with other games on
  (the closer game if two of your teams play at once). A game you pick still
  comes first; there's a checkbox to turn it off.

- **Spotlight ("primetime") mode.** When only one game is on (a Thursday
  night game, say), it fills the whole bottom of the screen in your look:
  the big score and clock, down and distance, who has the ball, the line
  score, and a strip with the last play and where it's on. Or pick any game
  to watch on the admin page. The ticker keeps running. On by default for
  single games; `--mock --spotlight` for screenshots. Protocol version 6.

- **Team logos from ESPN, if you want them.** A new setting in the admin
  page's Look section, off by default: Marqueet downloads the logos of the
  teams playing today from ESPN (the scores service), once each, and shows
  them on the LED ticker and in the widgets. Logos you add yourself win.

- The Snap Store listing has a title, a square app icon (generated from the
  parakeet dot grid like the other logo files, `media/marqueet-icon.svg`) and
  a link for reporting issues.

- **The Snap Store.** Every merge to `dev` that passes CI uploads to the Snap
  Store's `edge` channel. The installer installs from the store when it has a
  build (signed by the store and checked by snapd, which keeps Marqueet
  updated and can `snap revert` a bad update) and moves copies installed
  from a GitHub download over, keeping their settings. The GitHub download
  stays as a fallback (`MARQUEET_NO_STORE=1`).

- **Developer switch in the Pi image**: put a public key on the SD card as
  `marqueet-ssh-key.pub` and the Pi turns on SSH (keys only, no passwords, no
  root) for the Imager account at the next start; `marqueet-ssh-off` turns
  it off. See [docs/PI-DEVELOPMENT.md](docs/PI-DEVELOPMENT.md).

- **"Pick a look" in the welcome steps**: after sports, teams, town and
  fantasy, first-time setup shows the three looks as big tappable previews
  (skippable; Broadcast stays the default).

- **The header and score takeovers follow the look.** The header bar takes
  the look's colors and lettering (the league filter in Broadcast italics,
  Ballpark stencil or Varsity block letters; the clock in the highlight
  color). Takeovers: Broadcast keeps drifting team-color stripes; Ballpark
  shows the painted green board with bulb-yellow lettering; Varsity fills the
  area with the team's jersey color and a mesh, with a bar in the team's
  second color.

- **Your own team colors and logos**
  ([ADR-0013](docs/adr/0013-bring-your-own-team-art.md)). Marqueet still
  ships no logos, but you can add yours on the admin page: pick a team, set
  its colors, upload a PNG. Logos show as small lit-up logos on the LED ticker
  and next to the team in every look's widgets; colors replace the ones from
  the scores everywhere. **Team packs** (one JSON file for many teams) let you
  back up, move and share your setup; see
  [docs/TEAM_PACKS.md](docs/TEAM_PACKS.md). Protocol version 4.

- **Themes** for the crawl and widgets
  ([ADR-0012](docs/adr/0012-display-themes.md)). **Broadcast** (the new
  default) looks like TV score graphics: the game of the day split into the
  two teams' colors with a score box on the seam, score bugs for other games,
  a white bottom line for the crawl. **Ballpark** is a painted scoreboard with
  every number on a black plate. **Varsity** sets scores like jersey numbers.
  Each is a style plus nine colors by role; team colors come from the data
  and are kept readable automatically. Try them with
  `marqueet-display --mock --theme ballpark`.
- **Pick and build a look on the admin page**: a new Look section shows each
  theme as a small sketch in its own colors, a switch for team colors, and
  "Make your own colors" with a color picker per role, a warning when a pair
  would be hard to read, and a short code to share your look (paste one to use
  someone else's).

- **Welcome steps**: after creating the password on first setup, fans get
  four short, phone-friendly steps instead of the full admin page: which
  sports, which teams, your town (weather and storm warnings), and your
  Sleeper fantasy league. Town and fantasy can be skipped; everything can be
  changed later on the admin page.
- **Pick any team as a favorite**: the admin page lists every team in the
  leagues you follow (from ESPN's team lists, refreshed daily), not just
  today's; big leagues like college football get a "Find a team" box.
- **Raspberry Pi image**: download `marqueet-pi.img.xz`, flash it with
  Raspberry Pi Imager (optionally entering your WiFi in its settings), plug
  in the Pi: it installs itself on first boot
  (about 10 minutes, with plain progress on the screen, including "waiting
  for the internet" hints), checks for updates daily, has SSH off, and resets the
  admin password when you put a `reset-password` file on the SD card. Built
  from official Ubuntu Server 24.04 (signature-checked) with every build.
- **One-command install**: on Ubuntu 24.04 (Raspberry Pi 4/5, mini PC, old
  laptop), `curl -fsSL https://raw.githubusercontent.com/adamson34/marqueet/dev/install.sh | sudo sh`
  installs Ubuntu Frame and Marqueet, names the computer `marqueet`, and boots
  into the ticker; run it again to update. Latest builds are published
  automatically as the `edge` pre-release after every change.
- **A snap** (Phase 6, [ADR-0011](docs/adr/0011-packaging-snap.md)): one
  strictly confined `marqueet` snap with the server and display as daemons,
  running under Ubuntu Frame (it waits for Frame's Wayland socket, uses the
  `gpu-2404` Mesa drivers, keeps its data in `$SNAP_DATA`).
  `snap set marqueet reset-password=true` resets the admin password. CI builds
  it for amd64 and arm64; it isn't published yet. systemd units for
  from-source installs are in `packaging/systemd/`.
- **First-boot setup** (Phase 6): a new device with no admin password shows
  a setup screen: a QR code for the setup page, its address
  (`marqueet.local:7878/setup` and the IP), and a one-time 6-digit code in
  LED digits (the code is also in the server's log). Open the setup page
  from your phone or laptop, enter the code and choose a password: it's
  stored as a PBKDF2 hash, you're logged in, and the code stops working. Five
  wrong codes and a new one appears. Until then, other devices only see the
  setup page. `--reset-file PATH` puts the device back into setup when that
  file appears (for a forgotten password). `MARQUEET_ADMIN_PASSWORD` still
  works and skips setup.
- **Per-slot widget options**: each widget slot on the admin page can be
  narrowed: Game of the Day, Scores and Standings to one league, Fantasy to
  one of your followed teams (so two slots can show two leagues). The option
  picker appears under the widget it belongs to. Saved settings from before
  keep working.
- **Fantasy in takeovers**: when a touchdown (or other big play) involves one
  of your fantasy starters, its takeover says so: `YOUR STARTER | J. ALLEN
  24.1 PTS` (or `THEIR STARTER` for your opponent's). Players are matched
  through Sleeper's ESPN ids. With "favorites only" takeovers, your starters
  count as favorites.
- **Fantasy on the ticker and screen** (Phase 5): each followed matchup rides
  near the front of the ticker (`ALLEN WRENCH / MAHOMES ALONE  98.4 / 91.7`,
  the leader bright), flashing when your team takes or loses the lead. A
  Fantasy widget shows both totals in LED digits and every starter's points
  side by side by lineup slot. Team names are cleaned up for the LED font
  (emoji dropped).
- **Fantasy football setup** (Phase 5): on the admin page, enter your Sleeper
  username, pick a league and your team (up to four), and Marqueet follows the
  matchup: every 30 seconds while NFL games are on, every 10 minutes
  otherwise. Sleeper's player list is downloaded at most once a day and cached
  next to the database (`sleeper-players.json`, about 1 MB).
- **Widget layouts**: choose how the widget area is split on the admin page:
  wide left (the default), wide right, halves, three columns or a single
  widget, then pick a widget for each slot (drag a slot onto another to swap
  them). Widgets scale to fit narrower slots. `marqueet-display
  --widget-layout three` tries it with demo data.
- **Severe weather alerts** (US): with a location set, Marqueet checks the
  National Weather Service every 2 minutes. Alerts in effect lead the ticker
  in red (warnings), orange (watches) or amber (advisories) with an LED icon
  and when they end, e.g. `TORNADO WARNING / UNTIL 8:30 PM  JACKSON, MO + 2
  MORE`. A new severe warning takes over the screen; watches flash. On by
  default; turn it off on the admin page. Outside the US there's no data, and
  Marqueet stops asking.
- **Favorites' standings in the ticker**: each favorite team's place shows on
  its league's header, e.g. `NFL  BUF 1ST / AFC EAST 3-0` or
  `EPL  MUN 12TH / 5 PTS`, and in a small segment of its own on days that
  league has no games.
- **Feed API** ([docs/FEEDS.md](docs/FEEDS.md)): your own scripts, in any
  language, can put things on the sign. Create a feed on the admin page to get
  a token, then `POST /api/feeds/<name>` ticker segments (text, an optional
  second line, an LED icon, a color) and crawl lines, and
  `POST /api/feeds/<name>/alert` a flash or takeover. Content expires unless
  refreshed (15 minutes by default); feeds are limited in size and alert rate.
  Tokens are stored apart from the settings and never appear in
  `/api/settings`.
- `marqueet-display --server URL --screenshot out.png --wait 10` keeps
  listening in real time before capturing, so a live alert can be captured.
- **Takeovers without a score**: feed takeovers show a kicker, headline and
  detail line; `Takeover.score` is now optional.

- **Weather in the ticker**: once a location is set, each ticker loop opens
  with the weather on the LED sign: a color LED icon (sun, moon, clouds,
  rain, snow, storms, fog), the temperature, the city with today's high and
  low, and a red heads-up like `RAIN FRI / 80% CHANCE` when rain, snow or
  storms are 50%+ likely in the next few days. On by default; turn it off on
  the admin page. Ticker segments can now carry multi-color LED icons
  (`Part::Icon`, drawn from `assets/icons.txt`), so display protocol is now
  version 2.
- **Weather widget**: set a location on the admin page (a city name, looked
  up once, or "lat, lon") and °F or °C, then pick Weather for a widget slot.
  Shows current temperature, conditions, feels-like, wind and humidity, and
  five days of highs, lows and rain chances, with icons drawn on the card.
  Data from [Open-Meteo](https://open-meteo.com) (free, no API key, CC BY
  4.0), fetched every 15 minutes, and only while a weather widget is on
  screen: the location isn't sent anywhere otherwise.
- **Standings widget**: pick it for either widget slot on the admin page.
  It shows the group (division, conference or table) of your first favorite
  team, scrolled so that team is visible and highlighted, else the featured
  game's division, else the first league's first group. Columns fit the
  sport: W-L-T-PCT (football), W-L-PCT-GB (baseball, basketball),
  W-L-OTL-PTS (hockey), P-W-D-L-PTS (soccer). Standings come from ESPN's
  standings endpoint every 30 minutes (retry after 10 on failure); college
  leagues have none. `marqueet-display --mock --widgets game_of_the_day,standings`
  shows it with demo data.
- **Time zone setting** (admin page, `time_zone` in settings): an IANA name
  such as `America/Chicago`, with daylight saving, read from the system's
  zoneinfo. It drives start times in the ticker and crawl, the crawl's
  TONIGHT tag, quiet hours and the display's clock. Blank uses the device's
  time zone.
- **Admin page** (Phase 4) at `/admin`: leagues (tick and drag to order),
  favorite teams picked from today's games, takeover policy, widget slots,
  LED color, speeds, rows, glow, flicker and scroll mode, quiet hours, plus
  per-league fetch health and recent plays. Server-rendered HTML, plain CSS
  and one small hand-written script; no template engine, no front-end
  dependencies. Open without login on the device itself; from the network
  only when the server has an admin password (`MARQUEET_ADMIN_PASSWORD` or
  `--admin-password`, 8+ characters), which the server now requires before it
  will listen on a non-loopback address. Sessions are random tokens in an
  HttpOnly, SameSite=Strict cookie; cross-site form posts are refused; pages
  carry a strict Content-Security-Policy. `PUT /api/settings` follows the
  same rules.
- **Settings** (Phase 4): stored in SQLite (`marqueet-server --db`, default
  `marqueet.db`) and applied live: leagues and their order (pollers start and
  stop), favorite teams (they win Game of the Day), takeover policy (all,
  favorites only, or off, where the rest just flash), widget slots, the
  display look (sent to the display, which restyles on the fly), and
  overnight quiet hours (the screen blanks). `GET`/`PUT /api/settings`;
  changes are accepted only from the device itself until the admin page has a
  password. `--leagues` now overrides and saves the stored list.
- **Widgets** (Phase 4): the widget area now shows a **Game of the Day** card
  (big team names, LED-block scores, situation chips like "BUF ball / 2nd & 6
  / KC 14" or "1 out / On 1st & 3rd", and a line score) and a **Scores** card
  (live in red, breaks in amber, finals in green, upcoming muted). The server
  builds ready-to-draw views (`core::widgets`) and sends them with each
  content update; the display fades them in after the welcome logo and
  redraws only when they change (about 3 ms at 1080p on an M3). The LED clock
  in the widget area is gone; the header shows the time.
- **UI layer and header bar** (Phase 4): a CPU canvas with anti-aliased
  rounded rectangles, vector text (swash, bundled Barlow Condensed under the
  SIL OFL) and LED-block text, uploaded to the GPU only when it changes. The
  new header bar above the ticker shows a LIVE badge (grey when nothing is
  live), the league filter and an LED clock. `header_ratio` sizes it (0 hides
  it).
- **Takeovers** (Phase 3): touchdowns, home runs, grand slams and goals take
  over the widget area for 10 s: drifting team-color stripes with a dot
  texture, LED-block kicker, headline, play and score box (scoring side in
  amber), with fades. Takeovers queue one at a time and stale ones are
  dropped. Rendered with a new background shader plus an overlay mode of the
  LED renderer (square blocks, transparent background).
- `--score` (with `--mock`) now runs the real event engine, so it can trigger
  takeovers in screenshots and recordings.
- **Server alerts** (Phase 3): `marqueet-server` runs the event engine on
  every successful poll, dedupes alerts by id, and pushes them to connected
  displays right after the content update. `GET /api/alerts` lists the 50
  most recent. No alerts on a league's first snapshot or across a stale gap.
- **Event engine** (`marqueet-core::events`, Phase 3): compares consecutive
  snapshots of a game and reports touchdowns, field goals, safeties, extra
  points, two-point conversions, home runs, grand slams, runs, goals and finals
  (score corrections are detected but never alerted). Big plays become
  takeover alerts with a kicker, headline, play text and score line; the rest
  flash the ticker. Alert ids are deterministic so a moment is never shown
  twice. The `--mock` feed now uses the same engine.
- **Display live feed** (Phase 2): `marqueet-display` connects to
  `marqueet-server` (`--server URL`, default `ws://127.0.0.1:7878/ws`) on a
  background thread, shows CONNECTING TO SERVER until scores arrive, keeps the
  last scores up while disconnected, and reconnects with backoff. Built-in
  demo data moved behind `--mock`.
- **`marqueet-server`** (Phase 2): polls each league on an adaptive schedule
  (12 s while live, 60 s near kickoff, 5 min on game days, 20 min idle, with
  jitter and exponential backoff), keeps the last good data when ESPN fails
  and marks it DELAYED after 3 failures, formats games into ticker/crawl
  segments, and pushes them to the display over `ws://127.0.0.1:7878/ws`.
  Also `GET /api/games` (games plus per-league fetch health) and `GET /healthz`.
- **Wire protocol** (`marqueet-core::protocol`): `hello`, `content`, `alert`.
- **ESPN provider** (`marqueet-provider-espn`, Phase 2): fetches ESPN's
  unofficial scoreboards for NFL, college football, MLB, NBA, WNBA, college
  basketball, NHL, MLS, Premier League and Champions League, and normalizes
  them into the shared schema. Lenient per-event parsing (one bad game never
  hides the league), an honest `marqueet/<version>` user agent, and rustls with
  ring for TLS ([ADR-0009](docs/adr/0009-espn-provider.md)). Tested against
  real captures plus synthetic live states.
- **`DataProvider` interface** (`marqueet-core::provider`).
- **LED display on mock data (Phase 1).** `marqueet-display` is a native
  wgpu/winit app that draws a convincing LED sign: dot grid, glow, subtle
  flicker, stepped or smooth scrolling ([ADR-0004](docs/adr/0004-led-rendering-pipeline.md)).
  The main ticker shows two-line game blocks under league headers (possession,
  red zone, inning and outs, power plays, college ranks, dimmed losers); the
  crawl lists upcoming games; score changes flash the game's block.
- **Normalized sports schema** (`marqueet-core::sports`) covering football,
  basketball, baseball, hockey and soccer, with mock fixtures and ticker
  formatting.
- **Generic ticker segments and alerts** so non-sports sources can plug in
  later ([ADR-0003](docs/adr/0003-generic-segments-and-alerts.md)).
- **Hand-drawn LED font** as editable ASCII art, plus a Scale2x large font;
  tabular digits so scores don't jitter.
- **LED-safe team colors:** dark primaries are brightened keeping their hue.
- **Parakeet logo** drawn on a dot grid; SVGs in `media/` and the display's
  LED welcome screen are generated from one source
  ([ADR-0005](docs/adr/0005-dot-grid-brand-source-of-truth.md)).
- **Headless capture:** `--screenshot` for single frames and `--record` for
  frame sequences (for GIFs and videos).
- **Concept mockup GIF** (`media/mockup.gif`) of the planned design: the real
  LED ticker on top, with a header bar, flat crawl, widget cards (game of the
  day, scores) and a touchdown takeover composited below
  ([ADR-0007](docs/adr/0007-led-ticker-flat-widgets.md)).
- **`--score GAME:home|away:POINTS`** (headless) scripts a scoring alert: the
  game's score updates and its ticker block flashes in team color.
- **Repository setup:** CI (fmt, clippy, tests, ARM64 build, cargo-deny),
  Dependabot, issue and PR templates, CODEOWNERS, contributing and security
  policies, roadmap, ADRs, MIT license.

### Security

- `GET /api/settings` now follows the admin page's access rules (it includes
  the weather location).
- winit's Wayland client-side decorations are disabled, removing the
  unmaintained `ttf-parser` (RUSTSEC-2026-0192) from the dependency tree.
