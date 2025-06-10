.PHONY: fmt

fmt:
	cargo clippy --fix --allow-dirty
	cargo fix --allow-dirty
	cargo fmt
