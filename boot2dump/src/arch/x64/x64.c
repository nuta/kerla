#include "x64.h"
#include "printchar.h"
#include <printf.h>
#include <string.h>
#include <types.h>

static struct gdt gdt;
static struct idt idt;
static struct tss tss;
static struct gdtr gdtr;
static struct idtr idtr;

static void gdt_init(void) {
    uint64_t tss_addr = (uint64_t) &tss;
    gdt.null = 0x0000000000000000;
    gdt.kernel_cs = 0x00af9a000000ffff;
    gdt.tss_low =
        0x0000890000000000 | sizeof(struct tss) | ((tss_addr & 0xffff) << 16)
        | (((tss_addr >> 16) & 0xff) << 32) | (((tss_addr >> 24) & 0xff) << 56);
    gdt.tss_high = tss_addr >> 32;

    gdtr.laddr = (uint64_t) &gdt;
    gdtr.len = sizeof(gdt) - 1;
    asm_lgdt((uint64_t) &gdtr);
}

static void interrupt_handler(void) {
    PANIC("received an interrupt despite we've disabled it");
}

static void idt_init(void) {
    // Initialize IDT entries.
    for (int i = 0; i < IDT_DESC_NUM; i++) {
        uint64_t handler = (uint64_t) &interrupt_handler;
        idt.descs[i].offset1 = handler & 0xffff;
        idt.descs[i].seg = KERNEL_CS;
        idt.descs[i].ist = IST_RSP0;
        idt.descs[i].info = IDT_INT_HANDLER;
        idt.descs[i].offset2 = (handler >> 16) & 0xffff;
        idt.descs[i].offset3 = (handler >> 32) & 0xffffffff;
        idt.descs[i].reserved = 0;
    }

    idtr.laddr = (uint64_t) &idt;
    idtr.len = sizeof(idt) - 1;
    asm_lidt((uint64_t) &idtr);
}

// Disables PIC. We use IO APIC instead.
static void pic_init(void) {
    asm_out8(0xa1, 0xff);
    asm_out8(0x21, 0xff);
    asm_out8(0x20, 0x11);
    asm_out8(0xa0, 0x11);
    asm_out8(0x21, 0x20);
    asm_out8(0xa1, 0x28);
    asm_out8(0x21, 0x04);
    asm_out8(0xa1, 0x02);
    asm_out8(0x21, 0x01);
    asm_out8(0xa1, 0x01);
    asm_out8(0xa1, 0xff);
    asm_out8(0x21, 0xff);
}

static void tss_init(void) {
    tss.rsp0 = 0;
    tss.iomap_offset = offsetof(struct tss, iomap);
    tss.iomap_last_byte = 0xff;
    asm_ltr(TSS_SEG);
}

static void apic_init(void) {
    asm_wrmsr(MSR_APIC_BASE, (asm_rdmsr(MSR_APIC_BASE) & 0xfffff100) | 0x0800);
    write_apic(APIC_REG_SPURIOUS_INT, 1 << 8);
    write_apic(APIC_REG_TPR, 0);
    write_apic(APIC_REG_LOGICAL_DEST, 0x01000000);
    write_apic(APIC_REG_DEST_FORMAT, 0xffffffff);
    write_apic(APIC_REG_LVT_TIMER, 1 << 16 /* masked */);
    write_apic(APIC_REG_LVT_ERROR, 1 << 16 /* masked */);
}

extern char __bss[];
extern char __bss_end[];

void x64_init(void) {
    memset(__bss, 0, (vaddr_t) __bss_end - (vaddr_t) __bss);
    printchar_init();

    pic_init();
    apic_init();
    gdt_init();
    tss_init();
    idt_init();
}

void arch_halt(void) {
    __asm__ __volatile__("cli; hlt");
}

void arch_reboot(void) {
    // Cause a triple fault deliberately to reboot the computer.
    struct idtr empty_idtr;
    memset(&empty_idtr, 0, sizeof(empty_idtr));
    asm_lidt((uint64_t) &empty_idtr);
    asm_int3();
}
