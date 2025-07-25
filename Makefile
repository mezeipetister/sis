.PHONY: deploy

export NODE_OPTIONS=--openssl-legacy-provider

.PHONY: up all $(SUBDIRS) deploy

install_deps:
	echo "Installing dependencies..."
	rustup toolchain install stable --component rust-src
	cargo binstall espup --force
	cargo install espflash
	cargo install esp-generate
	cargo install ldproxy
deploy:
	