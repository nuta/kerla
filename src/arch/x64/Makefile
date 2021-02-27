QEMU 	  ?= qemu-system-x86_64
QEMUFLAGS += -m 512 -cpu IvyBridge,rdtscp -rtc base=utc -serial mon:stdio
QEMUFLAGS += -no-reboot -boot d -device isa-debug-exit -d guest_errors,unimp
QEMUFLAGS += $(if $(SMP), -smp $(SMP))
QEMUFLAGS += $(if $(GUI),,-nographic)

.PHONY: run
run: build
	cp $(kernel_image) penguin-kernel.x64-qemu.elf
	./tools/make-bootable-on-qemu.py penguin-kernel.x64-qemu.elf
	$(PROGRESS) "RUN" $<
	$(QEMU) $(QEMUFLAGS) -kernel penguin-kernel.x64-qemu.elf
