.PHONY: build test fmt clean deploy-testnet

# Build the contract WASM
build:
	cargo build --target wasm32-unknown-unknown --release
	@mkdir -p target/wasm32-unknown-unknown/release
	@echo "WASM artifact: target/wasm32-unknown-unknown/release/crate_marketplace.wasm"

# Run all tests
test:
	cargo test

# Format code
fmt:
	cargo fmt --all

# Run clippy lints
lint:
	cargo clippy --all-targets --all-features -- -D warnings

# Clean build artifacts
clean:
	cargo clean

# Deploy to testnet (requires stellar CLI + funded identity)
# Usage: make deploy-testnet IDENTITY=my-identity
deploy-testnet: build
	stellar contract deploy \
		--wasm target/wasm32-unknown-unknown/release/crate_marketplace.wasm \
		--source $(IDENTITY) \
		--network testnet

# Invoke upload_sample on testnet (example)
# Usage: make invoke-upload IDENTITY=my-identity
invoke-upload:
	stellar contract invoke \
		--id CA7DGEWWS3VH5J2I4I7FFEB5UHK2MJSYWDKDQKXQM7GDNLI2IRATDTLG \
		--source $(IDENTITY) \
		--network testnet \
		-- upload_sample \
		--uploader $(IDENTITY) \
		--title "Test Beat" \
		--ipfs_cid "QmTestCID" \
		--price_xlm 10 \
		--genre "Hip-Hop" \
		--bpm 90

# Get stats from deployed contract
stats:
	stellar contract invoke \
		--id CA7DGEWWS3VH5J2I4I7FFEB5UHK2MJSYWDKDQKXQM7GDNLI2IRATDTLG \
		--source $(IDENTITY) \
		--network testnet \
		-- get_stats
