#!/bin/sh
# Tests marqueet-tv-output.sh against a fake `snap` holding Ubuntu Frame's
# display config (the shape Frame prints for a 4K TV). Run: sh test.sh
set -eu
here=$(cd "$(dirname "$0")" && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir "$work/bin" "$work/common"
cat > "$work/frame.yaml" <<'YAML'
layouts:
  default:
    cards:
    - card-id: 0
      HDMI-A-1:
        # This output supports the following modes: 3840x2160@30.0, 3840x2160@30.0,
        # 3840x2160@30.0, 3840x2160@25.0, 2560x1440@60.0,
        # 1920x1080@60.0, 1920x1080@59.9, 1920x1080@50.0, 1920x1080@30.0,
        # 1280x720@60.0, 720x480@59.9
        #
        # Uncomment the following to enforce the selected configuration.
        state: enabled	# {enabled, disabled}, defaults to enabled
        mode: 3840x2160@30.0	# Defaults to preferred mode
        position: [0, 0]	# Defaults to [0, 0]
      HDMI-A-2:
        # (disconnected)
YAML
cat > "$work/bin/snap" <<SNAP
#!/bin/sh
case "\$1 \$2" in
  "get ubuntu-frame") cat "$work/frame.yaml" ;;
  "set ubuntu-frame") printf '%s\n' "\${3#display=}" > "$work/frame.yaml"; echo set >> "$work/sets" ;;
esac
SNAP
chmod +x "$work/bin/snap"
run() {
  printf '%s\n' "$1" > "$work/common/tv-output"
  PATH="$work/bin:$PATH" MARQUEET_COMMON="$work/common" sh "$here/marqueet-tv-output.sh" >/dev/null 2>&1
}
mode() { sed -nE 's/^[[:space:]]*mode:[[:space:]]*([^[:space:]#]+).*/\1/p' "$work/frame.yaml"; }
fail() { echo "FAIL: $*" >&2; exit 1; }

run 1920x1080@60
[ "$(mode)" = 1920x1080@60.0 ] || fail "auto should pick 1080p60, got $(mode)"
grep -q '^applied=1920x1080@60.0$' "$work/common/tv-output.status" || fail "status not written"
grep -q '^modes=3840x2160@30.0,3840x2160@25.0,2560x1440@60.0,' "$work/common/tv-output.status" || fail "modes not listed once each"
grep -q 'Defaults to preferred mode' "$work/frame.yaml" || fail "the rest of the config is kept"

run 3840x2160@60
[ "$(mode)" = 3840x2160@30.0 ] || fail "4K at 60 falls back to the best 4K refresh, got $(mode)"

run 1600x900@60
[ "$(mode)" = 1280x720@60.0 ] || fail "a size the TV lacks picks the biggest below it, got $(mode)"

run native
[ "$(mode)" = 3840x2160@30.0 ] || fail "native is the preferred (first) mode, got $(mode)"

sets=$(wc -l < "$work/sets")
run native
[ "$(wc -l < "$work/sets")" = "$sets" ] || fail "no change, no set"

run '1920x1080@60; rm -rf /'
[ "$(wc -l < "$work/sets")" = "$sets" ] || fail "a bad request is ignored"
# install.sh carries its own copy of the helper; it must be this one.
awk '/<<.TV_OUTPUT_SH.$/ { on = 1; next } /^TV_OUTPUT_SH$/ { on = 0 } on' "$here/../../install.sh" > "$work/embedded.sh"
cmp -s "$work/embedded.sh" "$here/marqueet-tv-output.sh" || fail "install.sh's copy of the helper differs; copy it over"
echo "tv-output: all good"
