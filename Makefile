build-release:
	cargo build -r && cp target/release/placecage-rust .

run:
	./placecage-rust