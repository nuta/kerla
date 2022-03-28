#ifndef __X64_H__
#define __X64_H__

#include <types.h>

//
//  Global Descriptor Table (GDT)
//
#define TSS_SEG   offsetof(struct gdt, tss_low)
#define KERNEL_CS offsetof(struct gdt, kernel_cs)

struct gdt {
    uint64_t null;
    uint64_t kernel_cs;
    uint64_t tss_low;
    uint64_t tss_high;
} __packed;

struct gdtr {
    uint16_t len;
    uint64_t laddr;
} __packed;

//
//  Interrupt Descriptor Table (IDT)
//
#define IDT_DESC_NUM    256
#define IDT_INT_HANDLER 0x8e
#define IST_RSP0        0

struct idt_desc {
    uint16_t offset1;
    uint16_t seg;
    uint8_t ist;
    uint8_t info;
    uint16_t offset2;
    uint32_t offset3;
    uint32_t reserved;
} __packed;

struct idt {
    struct idt_desc descs[IDT_DESC_NUM];
} __packed;

struct idtr {
    uint16_t len;
    uint64_t laddr;
} __packed;

//
//  PIC
//
#define PIT_HZ            1193182
#define PIT_CH2           0x42
#define PIT_CMD           0x43
#define KBC_PORT_B        0x61
#define KBC_B_OUT2_STATUS 0x20

//
//  APIC
//
#define APIC_REG_ID                     0xfee00020
#define APIC_REG_VERSION                0xfee00030
#define APIC_REG_TPR                    0xfee00080
#define APIC_REG_EOI                    0xfee000b0
#define APIC_REG_LOGICAL_DEST           0xfee000d0
#define APIC_REG_DEST_FORMAT            0xfee000e0
#define APIC_REG_SPURIOUS_INT           0xfee000f0
#define APIC_REG_ICR_LOW                0xfee00300
#define APIC_REG_ICR_HIGH               0xfee00310
#define APIC_REG_LVT_TIMER              0xfee00320
#define APIC_REG_LINT0                  0xfee00350
#define APIC_REG_LINT1                  0xfee00360
#define APIC_REG_LVT_ERROR              0xfee00370
#define APIC_REG_TIMER_INITCNT          0xfee00380
#define APIC_REG_TIMER_CURRENT          0xfee00390
#define APIC_REG_TIMER_DIV              0xfee003e0
#define IOAPIC_IOREGSEL_OFFSET          0x00
#define IOAPIC_IOWIN_OFFSET             0x10
#define VECTOR_IPI_RESCHEDULE           32
#define VECTOR_IPI_HALT                 33
#define VECTOR_IRQ_BASE                 48
#define IOAPIC_ADDR                     0xfec00000
#define IOAPIC_REG_IOAPICVER            0x01
#define IOAPIC_REG_NTH_IOREDTBL_LOW(n)  (0x10 + ((n) *2))
#define IOAPIC_REG_NTH_IOREDTBL_HIGH(n) (0x10 + ((n) *2) + 1)

//
//  Task State Segment (TSS)
//
#define TSS_IOMAP_SIZE 8191
struct tss {
    uint32_t reserved0;
    uint64_t rsp0;
    uint64_t rsp1;
    uint64_t rsp2;
    uint64_t reserved1;
    uint64_t ist[7];
    uint64_t reserved2;
    uint16_t reserved3;
    uint16_t iomap_offset;
    uint8_t iomap[TSS_IOMAP_SIZE];
    // According to Intel SDM, all bits of the last byte must be set to 1.
    uint8_t iomap_last_byte;
} __packed;

//
//  Model Specific Registers (MSR)
//
#define MSR_APIC_BASE 0x0000001b

static inline uint32_t read_apic(paddr_t addr) {
    return *((volatile uint32_t *) paddr2ptr(addr));
}

static inline void write_apic(paddr_t addr, uint32_t data) {
    *((volatile uint32_t *) paddr2ptr(addr)) = data;
}

// Disable clang-format temporarily because it does not handles "::" as desire.
// clang-format off

static inline void asm_out8(uint16_t port, uint8_t value) {
    __asm__ __volatile__("outb %0, %1" ::"a"(value), "Nd"(port));
}

static inline uint8_t asm_in8(uint16_t port) {
    uint8_t value;
    __asm__ __volatile__("inb %1, %0" : "=a"(value) : "Nd"(port));
    return value;
}

static inline void asm_lgdt(uint64_t gdtr) {
    __asm__ __volatile__("lgdt (%0)" :: "r"(gdtr));
}

static inline void asm_lidt(uint64_t idtr) {
    __asm__ __volatile__("lidt (%0)" :: "r"(idtr));
}

static inline void asm_ltr(uint16_t tr) {
    __asm__ __volatile__("ltr %0" :: "r"(tr));
}

static inline void asm_wrmsr(uint32_t reg, uint64_t value) {
    uint32_t low = value & 0xffffffff;
    uint32_t hi = value >> 32;
    __asm__ __volatile__("wrmsr" :: "c"(reg), "a"(low), "d"(hi));
}

static inline uint64_t asm_rdmsr(uint32_t reg) {
    uint32_t low, high;
    __asm__ __volatile__("rdmsr" : "=a"(low), "=d"(high) : "c"(reg));
    return ((uint64_t) high << 32) | low;
}

static inline void asm_int3(void) {
    __asm__ __volatile__("int3");
}

// clang-format on

#endif
