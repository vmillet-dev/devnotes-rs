#!/usr/bin/env bash
# Drives s1-shell under Xvfb: invisible creation, hide/reveal from the socket and from a global
# shortcut, AZERTY keys and dead keys, tray and portal dialog. Run from spikes/.
set -u
BIN=${BIN:-target/debug/s1-shell}
LOG=out/s1-x11.log
PROFILE=$(mktemp -d)
Xvfb :99 -screen 0 1920x1080x24 >/dev/null 2>&1 &
XVFB=$!
export DISPLAY=:99
sleep 1
eval "$(dbus-launch --sh-syntax)"
: > "$LOG"
until_log() { for _ in $(seq 1 100); do grep -q "$1" "$LOG" && return 0; sleep 0.1; done; echo "TIMEOUT waiting for $1" >> "$LOG"; return 1; }
state() { xwininfo -name "S1 shell" 2>/dev/null | grep "Map State" | awk '{print $3}'; }

$BIN --profile "$PROFILE" --tray >> "$LOG" 2>&1 &
APP=$!
until_log "first frame"
sleep 0.5
echo "## map state after first frame: $(state)" >> "$LOG"

$BIN --profile "$PROFILE" hide >> "$LOG" 2>&1; sleep 0.5
echo "## after socket hide: $(state)" >> "$LOG"
T0=$(date +%s%3N); $BIN --profile "$PROFILE" reveal >> "$LOG" 2>&1
for _ in $(seq 1 50); do [ "$(state)" = IsViewable ] && break; sleep 0.02; done
echo "## after second-instance reveal: $(state) in $(( $(date +%s%3N) - T0 )) ms" >> "$LOG"

$BIN --profile "$PROFILE" hide >> "$LOG" 2>&1; sleep 0.5
echo "## hidden again: $(state)" >> "$LOG"
xdotool key ctrl+shift+d; sleep 0.5
echo "## after global shortcut Ctrl+Shift+D: $(state)" >> "$LOG"

WIN=$(xdotool search --name "S1 shell" | head -1)
xdotool windowfocus --sync "$WIN" 2>/dev/null; sleep 0.3
# setxkbmap does not take under this Xvfb: the AZERTY keysyms are put on the keys under test.
xmodmap -pke > /tmp/qwerty.xmodmap
xmodmap -e "keycode 24 = a A" -e "keycode 38 = q Q" -e "keycode 25 = z Z" -e "keycode 52 = w W" \
        -e "keycode 10 = ampersand 1" -e "keycode 11 = eacute 2" -e "keycode 34 = dead_circumflex dead_diaeresis"
sleep 0.3
echo "## AZERTY: a z q 1/& 2/é ctrl+a ctrl+z, then dead ^ + e, dead ¨ + i" >> "$LOG"
for k in a z q ampersand shift+1 eacute ctrl+a ctrl+z; do xdotool key --window "$WIN" "$k"; sleep 0.1; done
xdotool key --window "$WIN" dead_circumflex e; sleep 0.1
xdotool key --window "$WIN" shift+dead_diaeresis i; sleep 0.3
xmodmap /tmp/qwerty.xmodmap; sleep 0.3
echo "## QWERTY: a z q 1 ctrl+a" >> "$LOG"
for k in a z q 1 ctrl+a; do xdotool key --window "$WIN" "$k"; sleep 0.1; done
sleep 0.3

$BIN --profile "$PROFILE" fullscreen >> "$LOG" 2>&1; sleep 0.5
echo "## after fullscreen: $(xwininfo -name 'S1 shell' | grep -E 'Width|Height' | tr -s ' ' | tr '\n' ' ')" >> "$LOG"
$BIN --profile "$PROFILE" quit >> "$LOG" 2>&1
wait $APP
echo "## exit code $?" >> "$LOG"

echo "## --- scale 1.5 and the portal dialog" >> "$LOG"
WINIT_X11_SCALE_FACTOR=1.5 $BIN --profile "$PROFILE" --dialog >> "$LOG" 2>&1 &
APP=$!
until_log "dialog answered"
sleep 0.5
$BIN --profile "$PROFILE" quit >> "$LOG" 2>&1
wait $APP
kill "$DBUS_SESSION_BUS_PID" "$XVFB" 2>/dev/null
rm -rf "$PROFILE"
