//! The TV's output mode. Marqueet runs confined and can't change the
//! screen's mode itself, so it writes the mode it wants (from the admin
//! page's resolution and frame rate) into its data folder, where a small
//! helper outside the snap (installed by `install.sh`, see
//! `packaging/tv-output/`) applies the closest mode the TV offers and writes
//! back what it did. Without the helper nothing happens: the display still
//! draws at the chosen resolution and scales up.

use std::path::PathBuf;

use marqueet_core::config::DisplayConfig;

const REQUEST: &str = "tv-output";
const STATUS: &str = "tv-output.status";

/// The snap's shared data folder, when running as a snap.
fn dir() -> Option<PathBuf> {
    std::env::var_os("SNAP_COMMON").map(PathBuf::from)
}

/// Asks for the TV mode that goes with `display` (only writing when it
/// changes).
pub fn request(display: &DisplayConfig) {
    let Some(dir) = dir() else { return };
    let want = display.resolution.tv_mode(display.max_fps);
    let path = dir.join(REQUEST);
    if std::fs::read_to_string(&path).is_ok_and(|now| now.trim() == want) {
        return;
    }
    match std::fs::write(&path, format!("{want}\n")) {
        Ok(()) => log::info!("asked for TV output {want}"),
        Err(e) => log::warn!("couldn't ask for TV output {want}: {e}"),
    }
}

/// What the helper last did: the mode applied and the modes the TV offers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Status {
    pub applied: String,
    pub modes: Vec<String>,
}

/// The helper's status, when it's installed and has run.
pub fn status() -> Option<Status> {
    parse(&std::fs::read_to_string(dir()?.join(STATUS)).ok()?)
}

/// Reads `applied=1920x1080@60.0` and `modes=3840x2160@30.0,...` lines.
pub fn parse(text: &str) -> Option<Status> {
    let value = |key: &str| text.lines().find_map(|l| l.trim().strip_prefix(key)?.strip_prefix('='));
    let applied = value("applied")?.trim().to_owned();
    let modes =
        value("modes").unwrap_or("").split(',').map(str::trim).filter(|m| !m.is_empty()).map(String::from).collect();
    (!applied.is_empty()).then_some(Status { applied, modes })
}

/// "1920x1080@60.0" as "1920×1080 at 60 Hz".
pub fn describe(mode: &str) -> String {
    match mode.split_once('@') {
        Some((size, hz)) => format!("{} at {} Hz", size.replace('x', "×"), hz.trim_end_matches(".0")),
        None => mode.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use marqueet_core::config::Resolution;

    #[test]
    fn modes_asked_for() {
        assert_eq!(Resolution::Auto.tv_mode(60), "1920x1080@60");
        assert_eq!(Resolution::P2160.tv_mode(30), "3840x2160@30");
        assert_eq!(Resolution::Native.tv_mode(60), "native");
    }

    #[test]
    fn the_helpers_status_reads_back() {
        let s = parse("applied=1920x1080@60.0\nmodes=3840x2160@30.0, 1920x1080@60.0,1280x720@60.0\n").unwrap();
        assert_eq!(s.applied, "1920x1080@60.0");
        assert_eq!(s.modes.len(), 3);
        assert_eq!(describe(&s.applied), "1920×1080 at 60 Hz");
        assert_eq!(describe("3840x2160@29.97"), "3840×2160 at 29.97 Hz");
        assert!(parse("nothing here").is_none());
        assert!(parse("applied=\n").is_none());
    }
}
