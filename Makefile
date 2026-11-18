# Common development commands: thin wrappers around cargo and tools/.
# What each step needs (targets, 32-bit libraries, your game folder): docs/GETTING-STARTED.md.

GAME   ?= $(CURDIR)/wine/gta-sa
TARGET := i686-unknown-linux-gnu
BIN    := target/$(TARGET)/release/sa-game
# 32-bit X11 client libraries for --window, when they are not installed system-wide
LIB32  ?= $(CURDIR)/target/play/lib32
RUN_ENV := $(if $(wildcard $(LIB32)),LD_LIBRARY_PATH=$(LIB32)$${LD_LIBRARY_PATH:+:$$LD_LIBRARY_PATH})
ARGS   ?=
TAG    ?= local
LANES  ?= 1,2,3

.DEFAULT_GOAL := help
.PHONY: help build build-mods run run-world run-samp web web-mods serve check test clippy verify frontier dupes structure bindings queue catalog progress

help: ## List the targets
	@awk 'BEGIN {FS = ":.*## "} /^[a-z0-9-]+:.*## / {printf "  \033[1m%-12s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)

build: ## Native sa-game (32-bit Linux, release)
	cargo build --release --target $(TARGET) -p sa-game

build-mods: ## Native sa-game with WebAssembly mods (SA:MP is the samp mod: docs/samp/README.md)
	cargo build --release --target $(TARGET) -p sa-game --features mods

run: build ## Run natively in a window (GAME=/path/to/gta-sa, ARGS=...)
	$(RUN_ENV) $(BIN) --game $(GAME) --window $(ARGS)

run-world: build ## Run natively, starting at Grove Street (skips the intro; exploration)
	$(RUN_ENV) SA_GAME_SCRIPT_SKIP=1 $(BIN) --game $(GAME) --window --start-in-world --system-heap $(ARGS)

run-samp: build-mods ## SA:MP as a mod, natively (the mod's connect screen picks the server)
	mods/build.sh
	$(RUN_ENV) $(BIN) --game $(GAME) --window --start-in-world --mods target/mods/resolution,target/mods/samp $(ARGS)

web: ## Browser build (wasm32) into web/pkg
	tools/web/build.sh

web-mods: ## Browser build with WebAssembly mods (SA:MP: ?mods=samp)
	FEATURES=mods tools/web/build.sh

serve: ## Serve the browser build on localhost:8642 (GAME=/path/to/gta-sa)
	tools/web/serve.sh --game $(GAME)

check: ## The writer gate: clippy (host, i686, wasm32), builds, hook tests, L1, queue, bindings, state.csv, structure
	tools/verify/writer-gate.sh

test: ## Fast tests: hook registration checks, core and SA:MP crates
	cargo test -p sa-hook --lib
	cargo test -p sa-core --lib
	cargo test -p sa-samp -p sa-samp-api

clippy: ## Host clippy of the whole workspace
	cargo clippy --workspace --all-targets -- -D warnings

verify: ## Replay every recording with the ports on (TAG=name LANES=1,2,3; Wine lanes)
	tools/e2e/verify-all.sh $(TAG) $(LANES)

frontier: ## Functions that ported code calls but that are not ported yet
	@test -f tools/verify/frontier.py || { echo "tools/verify/frontier.py is not on this branch yet"; exit 1; }
	python3 -I tools/verify/frontier.py

dupes: ## Functions defined in more than one crate (names, identical and renamed bodies)
	python3 -I tools/verify/dupes.py --bodies --renamed --summary

structure: ## The ADR-003 checks alone: bindings, copies across crates, platform cfgs, crate layers
	python3 -I tools/gen/bindings.py --check
	python3 -I tools/verify/dupes.py --check
	python3 -I tools/verify/cfg_check.py
	python3 -I tools/verify/crate_layers.py

bindings: ## Regenerate the orig! binding modules from catalog/bindings.csv (--check: BINDINGS_ARGS=--check)
	python3 -I tools/gen/bindings.py $(BINDINGS_ARGS)

queue: ## Regenerate the verification queue
	python3 -I tools/verify/queue.py

catalog: ## Regenerate the whole catalogue (needs the Ghidra project; about 2 minutes)
	tools/catalog/run_all.sh

progress: ## Show the port progress summary
	@sed -n '1,3p' catalog/progress.md
