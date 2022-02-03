# Default values for build system.
export V          ?=
export GUI        ?=
export RELEASE    ?=
export ARCH       ?= x64
export LOG        ?=
export LOG_SERIAL ?=
export CMDLINE    ?=
export QEMU_ARGS  ?=

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

# $(IMAGE): Use a Docker image for initramfs.
ifeq ($(IMAGE),)
INITRAMFS_PATH := build/testing.initramfs
export INIT_SCRIPT := /bin/sh
else
IMAGE_FILENAME := $(subst /,.s,$(IMAGE))
INITRAMFS_PATH := build/$(IMAGE_FILENAME).initramfs
export INIT_SCRIPT := $(shell tools/inspect-init-in-docker-image.py $(IMAGE))
endif

DUMMY_INITRAMFS_PATH := build/dummy-for-lint.initramfs

# Set the platform name for docker image cross compiling.
ifeq ($(ARCH),x64)
docker_platform = linux/amd64
endif

ifeq ($(docker_platform),)
$(error "docker_platform is not set for $(ARCH)!")
endif

topdir      := $(PWD)
build_mode  := $(if $(RELEASE),release,debug)
target_json := kernel/arch/$(ARCH)/$(ARCH).json
kernel_elf := kerla.$(ARCH).elf
stripped_kernel_elf := kerla.$(ARCH).stripped.elf
kernel_symbols := $(kernel_elf:.elf=.symbols)

PROGRESS   := printf "  \\033[1;96m%8s\\033[0m  \\033[1;m%s\\033[0m\\n"
PYTHON3    ?= python3
CARGO      ?= cargo
BOCHS      ?= bochs
NM         ?= rust-nm
READELF    ?= readelf
STRIP      ?= rust-strip
DRAWIO     ?= /Applications/draw.io.app/Contents/MacOS/draw.io

export RUSTFLAGS = -Z emit-stack-sizes
CARGOFLAGS += -Z build-std=core,alloc -Z build-std-features=compiler-builtins-mem
CARGOFLAGS += --target $(target_json)
CARGOFLAGS += $(if $(RELEASE),--release,)
TESTCARGOFLAGS += --package kerla_kernel -Z unstable-options
TESTCARGOFLAGS += --config "target.$(ARCH).runner = './tools/run-unittests.sh'"
WATCHFLAGS += --clear

export CARGO_FROM_MAKE=1
export INITRAMFS_PATH
export ARCH
export PYTHON3
export NM

#
#  Build Commands
#
.PHONY: build
build:
	$(MAKE) build-crate
	cp target/$(ARCH)/$(build_mode)/kerla_kernel $(kernel_elf)

	$(PROGRESS) "NM" $(kernel_symbols)
	$(NM) $(kernel_elf) | rustfilt | awk '{ $$2=""; print $$0 }' > $(kernel_symbols)

	$(PROGRESS) "SYMBOLS" $(kernel_elf)
	$(PYTHON3) tools/embed-symbol-table.py $(kernel_symbols) $(kernel_elf)

	$(PROGRESS) "STRIP" $(stripped_kernel_elf)
	$(STRIP) $(kernel_elf) -o $(stripped_kernel_elf)

.PHONY: build-crate
build-crate:
	$(MAKE) initramfs

	$(PROGRESS) "CARGO" "kernel"
	$(CARGO) build $(CARGOFLAGS) --manifest-path kernel/Cargo.toml

.PHONY: initramfs
initramfs: $(INITRAMFS_PATH)

.PHONY: buildw
buildw:
	$(CARGO) watch $(WATCHFLAGS) -s "$(MAKE) build-crate"

.PHONY: iso
iso: build
	$(PROGRESS) MKISO kerla.iso
	mkdir -p isofiles/boot/grub
	cp boot/grub.cfg isofiles/boot/grub/grub.cfg
	cp $(stripped_kernel_elf) isofiles/kerla.elf
	grub-mkrescue -o kerla.iso isofiles

.PHONY: run
run: build
	$(PYTHON3) tools/run-qemu.py                                           \
		--arch $(ARCH)                                                 \
		$(if $(GUI),--gui,)                                            \
		$(if $(KVM),--kvm,)                                            \
		$(if $(GDB),--gdb,)                                            \
		$(if $(LOG),--append-cmdline "log=$(LOG)",)                    \
		$(if $(CMDLINE),--append-cmdline "$(CMDLINE)",)                \
		$(if $(LOG_SERIAL),--log-serial "$(LOG_SERIAL)",)              \
		$(if $(QEMU),--qemu $(QEMU),)                                  \
		$(kernel_elf) -- $(QEMU_ARGS)

.PHONY: bochs
bochs: iso
	$(BOCHS) -qf boot/bochsrc

.PHONY: test
test:
	$(MAKE) initramfs
	$(CARGO) test $(CARGOFLAGS) $(TESTCARGOFLAGS)

.PHONY: testw
testw:
	$(CARGO) watch $(WATCHFLAGS) -s "$(MAKE) test"

.PHONY: check
check:
	$(MAKE) $(DUMMY_INITRAMFS_PATH)
	INITRAMFS_PATH=$(DUMMY_INITRAMFS_PATH) $(CARGO) check $(CARGOFLAGS)

.PHONY: checkw
checkw:
	$(CARGO) watch $(WATCHFLAGS) -s "$(MAKE) check"

.PHONY: docs
docs:
	$(PROGRESS) "MDBOOK" build/docs
	mkdir -p build
	make doc-images
	mdbook build -d $(topdir)/build/docs Documentation

.PHONY: doc-images
doc-images: $(patsubst %.drawio, %.svg, $(wildcard Documentation/*.drawio))

.PHONY: docsw
docsw:
	mkdir -p build
	mdbook serve -d $(topdir)/build/docs Documentation

.PHONY: src-docs
src-docs:
	RUSTFLAGS="-C panic=abort -Z panic_abort_tests" $(CARGO) doc

.PHONY: lint
lint:
	$(MAKE) $(DUMMY_INITRAMFS_PATH)
	INITRAMFS_PATH=$(DUMMY_INITRAMFS_PATH) RUSTFLAGS="-C panic=abort -Z panic_abort_tests" $(CARGO) clippy

.PHONY: strict-lint
strict-lint:
	$(MAKE) $(DUMMY_INITRAMFS_PATH)
	INITRAMFS_PATH=$(DUMMY_INITRAMFS_PATH) RUSTFLAGS="-C panic=abort -Z panic_abort_tests" $(CARGO) clippy -- -D warnings

.PHONY: lint-and-fix
lint-and-fix:
	$(MAKE) $(DUMMY_INITRAMFS_PATH)
	INITRAMFS_PATH=$(DUMMY_INITRAMFS_PATH) RUSTFLAGS="-C panic=abort -Z panic_abort_tests" $(CARGO) clippy --fix -Z unstable-options

.PHONY: print-stack-sizes
print-stack-sizes: build
	$(READELF) --stack-sizes $(kernel_elf) | sort -n | rustfilt

.PHONY: clean
clean:
	$(CARGO) clean
	rm -rf *.elf *.iso *.bin *.symbols isofiles

#
#  Build Rules
#
build/testing.initramfs: $(wildcard testing/*) $(wildcard testing/*/*) Makefile
	$(PROGRESS) "BUILD" testing
	cd testing && docker buildx build --platform $(docker_platform) -t kerla-testing .
	$(PROGRESS) "EXPORT" testing
	mkdir -p build
	$(PYTHON3) tools/docker2initramfs.py $@ kerla-testing

build/$(IMAGE_FILENAME).initramfs: tools/docker2initramfs.py Makefile
	$(PROGRESS) "EXPORT" $(IMAGE)
	mkdir -p build
	$(PYTHON3) tools/docker2initramfs.py $@ $(IMAGE)

$(DUMMY_INITRAMFS_PATH):
	mkdir -p $(@D)
	touch $@

%.svg: %.drawio
	$(PROGRESS) "DRAWIO" $@
	$(DRAWIO) -x -f svg -o $@ $<
