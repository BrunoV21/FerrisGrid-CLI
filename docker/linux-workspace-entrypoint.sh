#!/usr/bin/env sh
set -eu

display="${DISPLAY:-:99}"
screen="${XVFB_SCREEN:-1280x800x24}"
screen_width="${screen%%x*}"
screen_rest="${screen#*x}"
screen_height="${screen_rest%%x*}"
novnc_port="${NOVNC_PORT:-6080}"
vnc_port="${VNC_PORT:-5900}"

Xvfb "$display" -screen 0 "$screen" -ac +extension RANDR >/tmp/xvfb.log 2>&1 &
xvfb_pid="$!"

cleanup() {
  kill "$xvfb_pid" 2>/dev/null || true
}
trap cleanup INT TERM EXIT

for _ in $(seq 1 50); do
  if xdpyinfo -display "$display" >/dev/null 2>&1; then
    break
  fi
  sleep 0.1
done

openbox >/tmp/openbox.log 2>&1 &
x11vnc -display "$display" -localhost -nopw -forever -shared -rfbport "$vnc_port" >/tmp/x11vnc.log 2>&1 &
websockify --web=/usr/share/novnc/ "$novnc_port" "127.0.0.1:$vnc_port" >/tmp/novnc.log 2>&1 &

if command -v chromium >/dev/null 2>&1; then
  chromium \
    --no-sandbox \
    --disable-dev-shm-usage \
    --window-position=0,0 \
    --window-size="${screen_width},${screen_height}" \
    about:blank >/tmp/chromium.log 2>&1 &
fi

cat <<EOF
FerrisGrid Linux workspace is running.
- noVNC: http://127.0.0.1:${novnc_port}/vnc.html?autoconnect=1&resize=scale
- DISPLAY: ${display}
- backend: ${FERRISGRID_BACKEND:-native-linux-x11}

Try:
  ferrisgrid doctor
  ferrisgrid observe
EOF

if [ "$#" -gt 0 ]; then
  exec "$@"
fi

tail -f /tmp/xvfb.log /tmp/openbox.log /tmp/x11vnc.log /tmp/novnc.log
