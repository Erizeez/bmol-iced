# BMOL iced demo launcher.
#
# The demos live in the `liquid-glass-playground` package under
# `examples/playground`. Every target runs one binary; the first invocation
# compiles the shared dependencies.
#
#   make help         List every target
#   make iced         Settings + macOS traffic lights (reference demo)
#   make dock         Dock, search bar, icons and context menu
#   make controls     Traffic-light laboratory (all control states)
#   make context      Context menu / popover showcase
#   make comparison   Liquid-glass material comparison
#   make reference    Reference wallpaper / optics demo
#   make build        cargo check the whole workspace
#   make test         cargo test the whole workspace

SHELL      := /bin/bash
PLAYGROUND := -p liquid-glass-playground

.PHONY: help iced dock controls context comparison reference build test clean

help:
	@echo "BMOL iced demo launcher"
	@echo ""
	@echo "  make iced         liquid-glass-iced-demo            (Settings + traffic lights)"
	@echo "  make dock         liquid-glass-dock-demo            (Dock + search + context menu)"
	@echo "  make controls     liquid-glass-window-controls-demo (Traffic-light laboratory)"
	@echo "  make context      liquid-glass-context-menu-demo    (Context menu / popover)"
	@echo "  make comparison   liquid-glass-comparison-demo      (Material comparison)"
	@echo "  make reference    liquid-glass-reference-demo       (Reference optics)"
	@echo ""
	@echo "  make build        cargo check --workspace --all-targets"
	@echo "  make test         cargo test --workspace"
	@echo "  make clean        cargo clean"

iced:
	cargo run $(PLAYGROUND) --bin liquid-glass-iced-demo

dock:
	cargo run $(PLAYGROUND) --bin liquid-glass-dock-demo

controls:
	cargo run $(PLAYGROUND) --bin liquid-glass-window-controls-demo

context:
	cargo run $(PLAYGROUND) --bin liquid-glass-context-menu-demo

comparison:
	cargo run $(PLAYGROUND) --bin liquid-glass-comparison-demo

reference:
	cargo run $(PLAYGROUND) --bin liquid-glass-reference-demo

build:
	cargo check --workspace --all-targets

test:
	cargo test --workspace

clean:
	cargo clean
