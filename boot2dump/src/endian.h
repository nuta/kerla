#ifndef __ENDIAN_H__
#define __ENDIAN_H__

#include <types.h>

static inline uint16_t swap16(uint16_t x) {
    return ((x & 0xff00) >> 8) | ((x & 0x00ff) << 8);
}

static inline uint32_t swap32(uint32_t x) {
    return ((x & 0xff000000) >> 24) | ((x & 0x00ff0000) >> 8)
           | ((x & 0x0000ff00) << 8) | ((x & 0x000000ff) << 24);
}

static inline uint64_t swap64(uint64_t x) {
    return (((uint64_t) swap32(x & 0xffffffff)) << 32) | swap32(x >> 32);
}

#if __BYTE_ORDER__ == __ORDER_BIG_ENDIAN__
static inline uint16_t ntoh16(uint16_t x) {
    return x;
}
static inline uint32_t ntoh32(uint32_t x) {
    return x;
}
static inline uint16_t hton16(uint16_t x) {
    return x;
}
static inline uint32_t hton32(uint32_t x) {
    return x;
}
static inline uint16_t into_le16(uint16_t x) {
    return swap16(x);
}
static inline uint32_t into_le32(uint32_t x) {
    return swap32(x);
}
static inline uint16_t from_le16(uint16_t x) {
    return swap16(x);
}
static inline uint32_t from_le32(uint32_t x) {
    return swap32(x);
}
static inline uint64_t from_le32(uint64_t x) {
    return swap64(x);
}
#else
static inline uint16_t ntoh16(uint16_t x) {
    return swap16(x);
}
static inline uint32_t ntoh32(uint32_t x) {
    return swap32(x);
}
static inline uint16_t hton16(uint16_t x) {
    return swap16(x);
}
static inline uint32_t hton32(uint32_t x) {
    return swap32(x);
}
static inline uint16_t into_le16(uint16_t x) {
    return x;
}
static inline uint32_t into_le32(uint32_t x) {
    return x;
}
static inline uint64_t into_le64(uint64_t x) {
    return x;
}
static inline uint16_t from_le16(uint16_t x) {
    return x;
}
static inline uint32_t from_le32(uint32_t x) {
    return x;
}
static inline uint64_t from_le64(uint64_t x) {
    return x;
}
#endif

#endif
