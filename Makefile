# ── LemonCUC Build ───────────────────────────────────────────────────────────
CARGO        ?= cargo
PNPM         ?= pnpm
DOCKER       ?= docker

IMAGE_NAME   ?= lemoncuc
VERSION      ?= $(shell grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')

RUST_TARGET  := x86_64-unknown-linux-musl
RUST_BIN     := target/$(RUST_TARGET)/release/lemon-cuc-backend
DIST_DIR     := dist

.PHONY: all frontend backend docker run stop attach

# ── Default: build everything ─────────────────────────────────────────────────
all: frontend backend docker

# ── Frontend (noVNC + Vite) ──────────────────────────────────────────────────
frontend: $(DIST_DIR)/index.html

$(DIST_DIR)/index.html: $(wildcard src/vnc/*.ts src/vnc/**/*.ts src/*.ts index.html)
	$(PNPM) install --frozen-lockfile
	$(PNPM) run build

# ── Backend (Rust musl static binary) ────────────────────────────────────────
backend: $(RUST_BIN)

$(RUST_BIN): $(wildcard src/**/*.rs Cargo.toml Cargo.lock)
	$(CARGO) build --release --target $(RUST_TARGET)

# ── Docker image ─────────────────────────────────────────────────────────────
docker: backend
	$(DOCKER) build -t $(IMAGE_NAME):$(VERSION) -t $(IMAGE_NAME):latest .

# ── Run container ────────────────────────────────────────────────────────────
run:
	$(DOCKER) run --rm -d \
		--name $(IMAGE_NAME) \
		-p 6080:6080 \
		$(IMAGE_NAME):latest

# ── Stop container ───────────────────────────────────────────────────────────
stop:
	$(DOCKER) stop $(IMAGE_NAME)

# ── Attach to container logs ─────────────────────────────────────────────────
attach:
	$(DOCKER) logs -f $(IMAGE_NAME)

