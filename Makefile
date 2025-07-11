.PHONY: fmt

fmt:
	cargo clippy --fix --allow-dirty --all-targets --all-features -- -D warnings -W clippy::nursery -W clippy::pedantic
	cargo fix --allow-dirty
	cargo fmt
