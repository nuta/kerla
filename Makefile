# Default values for build system.
export V         ?=
export ARCH      ?= x64
export BUILD     ?= debug
export VERSION   ?= v0.8.0

# The default build target.
.PHONY: default
default: build

# Disable builtin implicit rules and variables.
MAKEFLAGS += --no-builtin-rules --no-builtin-variables
.SUFFIXES:

# Enable verbose output if $(V) is set.
ifeq ($(V),)
.SILENT:
endif

PROGRESS   := printf "  \\033[1;96m%8s\\033[0m  \\033[1;m%s\\033[0m\\n"
PYTHON3    ?= python3
CARGO      ?= cargo

CARGOFLAGS += -Z build-std=core,alloc -Z build-std-features=compiler-builtins-mem

target_json := src/arch/$(ARCH)/$(ARCH).json
kernel_image := penguin-kernel.$(ARCH).elf

include src/arch/$(ARCH)/Makefile

#
#  Build Commands
#
.PHONY: build
build:
	$(CARGO) build $(CARGOFLAGS) --target $(target_json)
	cp target/$(ARCH)/$(BUILD)/penguin-kernel $(kernel_image)

.PHONY: lint
lint:
	$(CARGO) lint

.PHONY: clean
clean:
	$(CARGO) clean
	rm -f *.elf
