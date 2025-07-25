# Makefile for building the project

# Default target
build:
	cargo build

build-release:
	cargo build --release

install:
	espup install

flash:
	espflash flash --monitor --partition-table partitions.csv --chip esp32 target/xtensa-esp32-espidf/debug/esp32 --no-stub

flash-release:
	espflash flash --monitor --partition-table partitions.csv --chip esp32 target/xtensa-esp32-espidf/release/esp32 --no-stub