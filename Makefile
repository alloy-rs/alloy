fmt:
	cargo +nightly fmt --all

clippy:
	cargo +nightly clippy --all-features

check:
	cargo check --all-features

build:
	cargo build --all-features

test:
	cargo test --all-features && \
	cargo test --no-default-features

clean:
	cargo clean

pr:
	make fmt && \
	make clippy


.PHONY: fmt clippy check build test clean