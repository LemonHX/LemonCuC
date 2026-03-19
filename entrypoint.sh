#!/bin/bash
set -e

# ── Configurable parameters (override via docker run -e) ─────────────────────
SCREEN_WIDTH="${SCREEN_WIDTH:-1280}"
SCREEN_HEIGHT="${SCREEN_HEIGHT:-800}"
SCREEN_DEPTH="${SCREEN_DEPTH:-24}"
VNC_PORT="${VNC_PORT:-5900}"
VNC_PASSWORD="${VNC_PASSWORD:-}"

# ── Clean up child processes on exit ─────────────────────────────────────────
cleanup() {
    echo "entrypoint: shutting down..."
    kill 0 2>/dev/null || true
    wait
}
trap cleanup EXIT SIGTERM SIGINT

# ── Virtual display ───────────────────────────────────────────────────────────
Xvfb :0 -screen 0 "${SCREEN_WIDTH}x${SCREEN_HEIGHT}x${SCREEN_DEPTH}" \
    -ac +extension GLX +render -noreset &
XVFB_PID=$!
sleep 1

# Disable X-level screen-saver / blanking (skip -dpms, Xvfb has no DPMS)
xset s off s noblank 2>/dev/null || true

# ── D-Bus system + session bus (no systemd) ──────────────────────────────────
mkdir -p /run/dbus
dbus-daemon --system --fork 2>/dev/null || true
eval "$(dbus-launch --sh-syntax)"
export DBUS_SESSION_BUS_ADDRESS

# ── PulseAudio (virtual sound server) ─────────────────────────────────
pulseaudio --start --exit-idle-time=-1
# Load a virtual null sink so apps have somewhere to output audio.
# The .monitor source lets tcpulse capture everything being played.
pactl load-module module-null-sink sink_name=virtual_speaker sink_properties=device.description="VirtualSpeaker" 2>/dev/null || true
pactl set-default-sink virtual_speaker 2>/dev/null || true

# ── XFCE4 desktop ─────────────────────────────────────────────────────────────
startxfce4 &
sleep 2

# Disable XFCE4 screensaver and power manager via xfconf-query
xfconf-query -c xfce4-power-manager -p /xfce4-power-manager/dpms-enabled          -s false 2>/dev/null || true
xfconf-query -c xfce4-power-manager -p /xfce4-power-manager/blank-on-ac           -s 0     2>/dev/null || true
xfconf-query -c xfce4-power-manager -p /xfce4-power-manager/dpms-on-ac-sleep      -s 0     2>/dev/null || true
xfconf-query -c xfce4-power-manager -p /xfce4-power-manager/dpms-on-ac-off        -s 0     2>/dev/null || true

# ── VNC server ────────────────────────────────────────────────────────────────
if [ -n "${VNC_PASSWORD}" ]; then
    mkdir -p ~/.vnc
    x11vnc -storepasswd "${VNC_PASSWORD}" ~/.vnc/passwd
    x11vnc -display :0 -forever -shared -rfbauth ~/.vnc/passwd -rfbport "${VNC_PORT}" &
else
    x11vnc -display :0 -forever -shared -nopw -rfbport "${VNC_PORT}" &
fi
sleep 1

# ── LemonCUC backend (websockify + tcpulse) ──────────────────────────────────
# Run as PID 1's direct child — if it exits, the container exits.
echo "entrypoint: starting lemon-cuc-backend"
exec lemon-cuc-backend