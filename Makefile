CARGO := cargo
BIN := worklog

.PHONY: build run check lint fmt clean

build:
	$(CARGO) build --bin $(BIN) --release

run:
	$(CARGO) run --bin $(BIN) --release

check:
	$(CARGO) check --all-targets

lint:
	$(CARGO) clippy --all-targets --all-features -- -D warnings

fmt:
	$(CARGO) fmt --all

clean:
	$(CARGO) clean
