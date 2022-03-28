TEST_LDFLAGS += --script=test/x64/test_x64.ld

QEMU ?= qemu-system-x86_64
QEMUFLAGS += -m 512 -cpu IvyBridge,rdtscp
