FROM debian:13

# replace shell with bash so we can source files
RUN rm /bin/sh && ln -s /bin/bash /bin/sh

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
    # Window manager (i3)
    i3-wm \
    i3status \
    suckless-tools \
    # Notification daemon
    dunst \
    # File manager
    thunar \
    # VNC
    x11vnc \
    # SSH server
    openssh-server \
    # PolicyKit
    polkitd \
    # GTK / GLib / GDK runtime
    libgtk-3-0 \
    libgtk-4-1 \
    libglib2.0-0 \
    libgdk-pixbuf-2.0-0 \
    adwaita-icon-theme \
    # Cursor
    xcursor-themes \
    # Terminal emulator (for i3-sensible-terminal)
    xterm \
    # X fonts for xterm
    xfonts-base \
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
    jq \
    iproute2 \
    net-tools \
    netcat-openbsd \
    curl \
    wget \
    git \
    sudo \
    tree \
    ffmpeg

# ── machine-id (needed by D-Bus / Chrome) ────────────────────────────────────
RUN dbus-uuidgen > /etc/machine-id && cp /etc/machine-id /var/lib/dbus/machine-id

# ── Locale ────────────────────────────────────────────────────────────────────
RUN sed -i 's/# en_US.UTF-8/en_US.UTF-8/' /etc/locale.gen && locale-gen

# ── Font configuration ───────────────────────────────────────────────────────
COPY files/99-lemoncuc-fonts.conf /etc/fonts/conf.d/99-lemoncuc-fonts.conf
RUN fc-cache -fv

# ── CA certificates for secure downloads ───────────────────────────────────────
RUN apt-get install -y --no-install-recommends ca-certificates && update-ca-certificates

# ── Google Chrome (via direct download) ───────────────────────────────────────
RUN curl -fsSL "https://dl.google.com/linux/direct/google-chrome-stable_current_amd64.deb" -o /tmp/chrome.deb \
    && apt-get install -y --no-install-recommends /tmp/chrome.deb \
    && rm /tmp/chrome.deb

# Replace Chrome wrappers with container-safe versions
RUN mv /usr/bin/google-chrome-stable /usr/bin/google-chrome-stable-old \
    && mv /usr/bin/google-chrome       /usr/bin/google-chrome-old
COPY files/google-chrome-stable /usr/bin/google-chrome-stable
COPY files/google-chrome        /usr/bin/google-chrome
RUN chmod +x /usr/bin/google-chrome-stable /usr/bin/google-chrome

# Install UV
ADD https://astral.sh/uv/install.sh /uv-installer.sh
RUN sh /uv-installer.sh && rm /uv-installer.sh
ENV PATH="/root/.local/bin/:$PATH"

# Install NVM and Node.js and global npm packages (skills + agent-browser CLIs, plus their dependencies)
ENV NVM_DIR=/root/.nvm
RUN curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.4/install.sh | PROFILE="${BASH_ENV}" bash
RUN source $NVM_DIR/nvm.sh && \
    nvm install --lts && \
    nvm alias default lts/* && \
    nvm use default && \
    npm install -g skills agent-browser && \
    npx skills add vercel-labs/skills -a cline -s find-skills -g -y && \
    npx skills add vercel-labs/agent-browser -a cline -s agent-browser -g -y

# ── SSH: generate host keys + root login key (passwordless local SSH) ────────
RUN mkdir -p /run/sshd \
    && ssh-keygen -A \
    && ssh-keygen -t ed25519 -f /root/.ssh/id_ed25519 -N "" \
    && cat /root/.ssh/id_ed25519.pub >> /root/.ssh/authorized_keys \
    && chmod 600 /root/.ssh/authorized_keys
COPY files/sshd_config /etc/ssh/sshd_config

# ── LemonCUC backend (pre-built musl static binary) ──────────────────────────
COPY target/x86_64-unknown-linux-musl/release/lemon-cuc-backend /usr/local/bin/lemon-cuc-backend
RUN chmod +x /usr/local/bin/lemon-cuc-backend

# ── Docker CLI (static binary) ───────────────────────────────────────────────
ARG DOCKER_VERSION=29.3.0
RUN curl -fsSL "https://download.docker.com/linux/static/stable/x86_64/docker-${DOCKER_VERSION}.tgz" \
    | tar -xz --strip-components=1 -C /usr/local/bin docker/docker \
    && chmod +x /usr/local/bin/docker

# ── i3 + dunst + properties configuration ────────────────────────────────────
RUN mkdir -p /root/.config/i3 /root/.config/i3status /root/.config/dunst
COPY files/i3-config     /root/.config/i3/config
COPY files/i3status.conf /root/.config/i3status/config
COPY files/dunstrc       /root/.config/dunst/dunstrc
COPY files/properties.json /etc/lemoncuc/properties.json

# ── Entrypoint ────────────────────────────────────────────────────────────────
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

EXPOSE 6080

CMD ["/entrypoint.sh"]
