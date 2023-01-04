SHELL := bash
.SHELLFLAGS := -eu -o pipefail -c
.ONESHELL:
.DELETE_ON_ERROR:
MAKEFLAGS += --warn-undefined-variables
MAKEFLAGS += --no-builtin-rules
ifeq ($(origin .RECIPEPREFIX), undefined)
  $(error This Make does not support .RECIPEPREFIX; Please use GNU Make 4.0 or later)
endif
# The editor config for IDEs automatically converts tabs (default Make config) to spaces. Use a printable character instead of whitespace.
.RECIPEPREFIX = >
THIS_MAKEFILE_PATH:=$(word $(words $(MAKEFILE_LIST)),$(MAKEFILE_LIST))
THIS_DIR:=$(shell cd $(dir $(THIS_MAKEFILE_PATH));pwd)
THIS_MAKEFILE:=$(notdir $(THIS_MAKEFILE_PATH))

# Uncommit the following if more actions are added to this Makefile .
#usage:
#> @grep -E '(^[a-zA-Z_-]+:\s*?##.*$$)|(^##)' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.?## "}; {printf "\033[32m %-30s\033[0m%s\n", $$1, $$2}' | sed -e 's/\[32m ## /[33m/'
#.PHONY: usage
#.SILENT: usage

## Build

TARGET := x86_64-unknown-linux-gnu
BINARIES := echo primes keystore chat db mob
shrink: ## Build Tiny Distibution Binaries
shrink:
> rustup target add "$(TARGET)"
> rustup toolchain install nightly
> rustup component add rust-src --toolchain nightly
> cargo +nightly build -Z "build-std=std,panic_abort" --target "$(TARGET)" --release
> for BINARY in $(BINARIES); do
>     rm -f "$(THIS_DIR)/target/$${BINARY}"
>     upx --best -o "$(THIS_DIR)/target/$${BINARY}" "$(THIS_DIR)/target/$(TARGET)/release/$${BINARY}"
> done
.PHONY: shrink
.SILENT: shrink
