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

target_json := src/arch/$(ARCH)/$(ARCH).json
kernel_elf := penguin-kernel.$(ARCH).elf

PROGRESS   := printf "  \\033[1;96m%8s\\033[0m  \\033[1;m%s\\033[0m\\n"
PYTHON3    ?= python3
CARGO      ?= cargo

CARGOFLAGS += -Z build-std=core,alloc -Z build-std-features=compiler-builtins-mem
CARGOFLAGS += --target $(target_json)
TESTCARGOFLAGS += -Z unstable-options
TESTCARGOFLAGS += --config "target.$(ARCH).runner = '$(PYTHON3) tools/run-qemu.py --arch $(ARCH)'"

#
#  Build Commands
#
.PHONY: build
build:
	$(CARGO) build $(CARGOFLAGS)
	cp target/$(ARCH)/$(BUILD)/penguin-kernel $(kernel_elf)

.PHONY: run
run: build
	$(PYTHON3) tools/run-qemu.py --arch $(ARCH) $(kernel_elf)

.PHONY: test
test:
	$(CARGO) test $(CARGOFLAGS) $(TESTCARGOFLAGS)

.PHONY: testw
testw:
	$(CARGO) watch -s "$(MAKE) test"

.PHONY: lint
lint:
	$(CARGO) lint

.PHONY: clean
clean:
	$(CARGO) clean
	rm -f *.elf
