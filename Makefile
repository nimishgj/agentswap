.PHONY: test lint ci build

build:
	cargo build

test:
	cargo test

lint:
	cargo clippy -- -D warnings
	cargo fmt --check

ci: lint test
