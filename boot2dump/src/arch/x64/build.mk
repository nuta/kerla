LDFLAGS += --script=src/arch/x64/x64.ld
CFLAGS += --target=x86_64 -mcmodel=small -fno-omit-frame-pointer
CFLAGS += -mno-red-zone -mno-mmx -mno-sse -mno-sse2 -mno-avx -mno-avx2
