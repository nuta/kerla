#ifndef __ARCH_TYPES_H__
#define __ARCH_TYPES_H__

#define STRAIGHT_MAPPING_BASE 0xffff800000000000
#define PAGE_SIZE             4096
#define mb()                  __asm__ __volatile__("mfence")

static inline void *paddr2ptr(paddr_t paddr) {
    return (void *) (paddr | STRAIGHT_MAPPING_BASE);
}

static inline paddr_t vaddr2paddr(vaddr_t vaddr) {
    return vaddr & ~STRAIGHT_MAPPING_BASE;
}

static inline paddr_t ptr2paddr(void *ptr) {
    return vaddr2paddr((vaddr_t) ptr);
}

static inline void ioport_write8(uint16_t port, uint8_t value) {
    __asm__ __volatile__("outb %0, %1" ::"a"(value), "Nd"(port));
}

static inline void ioport_write16(uint16_t port, uint16_t value) {
    __asm__ __volatile__("outw %0, %1" ::"a"(value), "Nd"(port));
}

static inline void ioport_write32(uint16_t port, uint32_t value) {
    __asm__ __volatile__("outl %0, %1" ::"a"(value), "Nd"(port));
}

static inline uint8_t ioport_read8(uint16_t port) {
    uint8_t value;
    __asm__ __volatile__("inb %1, %0" : "=a"(value) : "Nd"(port));
    return value;
}

static inline uint16_t ioport_read16(uint16_t port) {
    uint16_t value;
    __asm__ __volatile__("inw %1, %0" : "=a"(value) : "Nd"(port));
    return value;
}

static inline uint32_t ioport_read32(uint16_t port) {
    uint32_t value;
    __asm__ __volatile__("inl %1, %0" : "=a"(value) : "Nd"(port));
    return value;
}

void arch_printchar(char ch);
void arch_halt(void);
void arch_reboot(void);

#endif
