CONTRACT_DIR := fixtures/testdata-contract
WASM_TARGET  := wasm32v1-none
FIXTURE      := tests/fixtures/testdata.wasm

.PHONY: all build test lint fmt check clean fixtures

## Everything CI checks.
all: check

build:
	cargo build

test:
	cargo test

lint:
	cargo clippy --all-targets -- -D warnings

fmt:
	cargo fmt --all

check:
	cargo fmt --all --check
	cargo clippy --all-targets -- -D warnings
	cargo test

## Rebuild the test fixture from fixtures/testdata-contract.
##
## Requires the wasm target: rustup target add wasm32v1-none
fixtures:
	cd $(CONTRACT_DIR) && cargo build --release --target $(WASM_TARGET)
	cp $(CONTRACT_DIR)/target/$(WASM_TARGET)/release/lumenscan_testdata.wasm $(FIXTURE)
	@echo "rebuilt $(FIXTURE)"

clean:
	cargo clean
	cd $(CONTRACT_DIR) && cargo clean
