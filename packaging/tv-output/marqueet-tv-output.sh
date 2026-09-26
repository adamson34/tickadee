#!/bin/sh
# Marqueet's TV output helper. Marqueet runs confined and can't change the
# screen's mode itself: it writes the mode it wants ("1920x1080@60", or
# "native") to its data folder, and this, run by systemd as root when that
# file changes and at boot, picks the closest mode the TV offers and sets it
# in Ubuntu Frame. It writes back what it applied, and the modes the TV
# offers, for the admin page.
set -eu

dir=${MARQUEET_COMMON:-/var/snap/marqueet/common}
[ -f "$dir/tv-output" ] || exit 0
want=$(head -c 64 "$dir/tv-output" | tr -d '[:space:]')
case $want in
  native) ;;
  *)
    # Only a plain WIDTHxHEIGHT@HZ: this runs as root on a file the snap writes.
    if ! printf '%s' "$want" | grep -Eqx '[0-9]{3,4}x[0-9]{3,4}@[0-9]{2,3}'; then
      echo "marqueet: ignoring TV mode request '$want'" >&2
      exit 0
    fi
    ;;
esac

config=$(snap get ubuntu-frame display 2>/dev/null) || exit 0

# The modes the connected screen offers, from Frame's own comment
# ("This output supports the following modes: 3840x2160@30.0, ..."), once each.
modes=$(printf '%s\n' "$config" | awk '
  /supports the following modes:/ { on = 1 }
  on {
    n = split($0, a, /[ ,#]+/); got = 0
    for (i = 1; i <= n; i++) if (a[i] ~ /^[0-9]+x[0-9]+@[0-9.]+$/) { print a[i]; got = 1 }
    if (!got && $0 !~ /supports/) exit
  }' | awk '!seen[$0]++')
[ -n "$modes" ] || exit 0

if [ "$want" = native ]; then
  # Frame lists the screen's preferred mode first.
  pick=$(printf '%s\n' "$modes" | head -n 1)
else
  size=${want%@*}
  pick=$(printf '%s\n' "$modes" | awk -F'[x@]' -v w="${size%x*}" -v h="${size#*x}" -v f="${want#*@}" '
    {
      mw = $1 + 0; mh = $2 + 0; mf = $3 + 0
      # That size at the best refresh up to the one asked for...
      if (mw == w && mh == h && mf <= f + 0.5 && mf > best) { best = mf; exact = $0 }
      # ...or else the biggest no taller than asked, at the best refresh.
      if (mh <= h && mf <= f + 0.5 && (mw * mh > area || (mw * mh == area && mf > fb))) {
        area = mw * mh; fb = mf; fallback = $0
      }
    }
    END { print (exact != "" ? exact : fallback) }')
fi
[ -n "$pick" ] || exit 0

current=$(printf '%s\n' "$config" | sed -nE 's/^[[:space:]]*mode:[[:space:]]*([^[:space:]#]+).*/\1/p' | head -n 1)
if [ "$pick" != "$current" ]; then
  new=$(printf '%s\n' "$config" | sed -E "s/^([[:space:]]*)mode:[[:space:]]*[^[:space:]#]+/\1mode: $pick/")
  snap set ubuntu-frame display="$new"
  echo "marqueet: TV output $current -> $pick"
fi
{
  echo "applied=$pick"
  echo "modes=$(printf '%s\n' "$modes" | paste -sd, -)"
} > "$dir/tv-output.status"
