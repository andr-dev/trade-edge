.PHONY: run

money:
	RUSTFLAGS="-C target-cpu=native" cargo run -p trade-edge-bot --release

dash:
	cargo run -p trade-edge-tui --release
