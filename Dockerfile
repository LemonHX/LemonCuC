FROM debian:13

ENV DEBIAN_FRONTEND=noninteractive \
    DEBIAN_PRIORITY=high \
    DISPLAY=:0 \
    LANG=en_US.UTF-8

# ── System base ──────────────────────────────────────────────────────────────
RUN apt-get update && apt-get install -y --no-install-recommends \
    # Virtual display
    xvfb \
    x11-xserver-utils \
    x11-utils \
    xauth \
    # Desktop environment
    xfce4 \
    xfce4-goodies \
    xfce4-terminal \
    # VNC
    x11vnc \
    # Input / screenshot
    xdotool \
    scrot \
    # Fonts
    fonts-liberation \
    fonts-noto \
    fonts-noto-cjk \
    fonts-noto-cjk-extra \
    fonts-noto-color-emoji \
    fontconfig \
    # Locale
    locales \
    # D-Bus (session + system bus, no systemd)
    dbus \
    dbus-x11 \
    # Thunar thumbnails
    tumbler \
    # Building
    build-essential\
    # Audio (PulseAudio virtual sink + GStreamer)
    pulseaudio \
    gstreamer1.0-tools \
    gstreamer1.0-plugins-base \
    gstreamer1.0-plugins-good \
    gstreamer1.0-plugins-bad \
    gstreamer1.0-pulseaudio \
    # Utilities
    net-tools \
    netcat-openbsd \
    curl \
    wget \
    git \
    sudo \
    tree \
    ffmpeg

# ── Locale ────────────────────────────────────────────────────────────────────
# ── machine-id (needed by D-Bus / Chrome) ────────────────────────────────────
RUN dbus-uuidgen > /etc/machine-id && cp /etc/machine-id /var/lib/dbus/machine-id

RUN sed -i 's/# en_US.UTF-8/en_US.UTF-8/' /etc/locale.gen && locale-gen

# ── Font configuration ───────────────────────────────────────────────────────
# Set Noto Sans as default sans-serif, with CJK fallback and subpixel hinting
COPY files/99-lemoncuc-fonts.conf /etc/fonts/conf.d/99-lemoncuc-fonts.conf
RUN fc-cache -fv

# ── CA certificates for secure downloads ───────────────────────────────────────
RUN apt-get install -y --no-install-recommends ca-certificates && update-ca-certificates

# ── Google Chrome (via direct download) ───────────────────────────────────────
RUN curl -fsSL "https://dl.google.com/linux/direct/google-chrome-stable_current_amd64.deb" -o /tmp/chrome.deb \
    && apt-get install -y --no-install-recommends /tmp/chrome.deb \
    && rm /tmp/chrome.deb

# ── LemonCUC backend (pre-built musl static binary) ──────────────────────────
COPY target/x86_64-unknown-linux-musl/release/lemon-cuc-backend /usr/local/bin/lemon-cuc-backend
RUN chmod +x /usr/local/bin/lemon-cuc-backend

# ── Docker CLI (static binary) ───────────────────────────────────────────────
# Pre-installed so DockerAccess works without any runtime download.
ARG DOCKER_VERSION=29.3.0
RUN curl -fsSL "https://download.docker.com/linux/static/stable/x86_64/docker-${DOCKER_VERSION}.tgz" \
    | tar -xz --strip-components=1 -C /usr/local/bin docker/docker \
    && chmod +x /usr/local/bin/docker

# ── Desktop config files ──────────────────────────────────────────────────────
RUN mkdir -p /root/Desktop
COPY files/google-chrome.desktop /usr/share/applications/google-chrome.desktop
COPY files/google-chrome.desktop /root/Desktop/google-chrome.desktop
RUN chmod +x /root/Desktop/google-chrome.desktop

# Autostart entry to disable screensaver / blanking once X is up
COPY files/screensaver.desktop /etc/xdg/autostart/disable-screensaver.desktop

# ── Entrypoint ────────────────────────────────────────────────────────────────
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

EXPOSE 6080 5702

CMD ["/entrypoint.sh"]
