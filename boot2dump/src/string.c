#include "string.h"

size_t strlen(const char *s) {
    size_t len = 0;
    while (*s != '\0') {
        len++;
        s++;
    }
    return len;
}

int strcmp(const char *s1, const char *s2) {
    while (*s1 == *s2 && *s1 != '\0' && *s2 != '\0') {
        s1++;
        s2++;
    }

    return *s1 - *s2;
}

int strncmp(const char *s1, const char *s2, size_t len) {
    while (len > 0) {
        if (*s1 != *s2) {
            return *s1 - *s2;
        }

        if (*s1 == '\0') {
            // Both `*s1` and `*s2` equal to '\0'.
            break;
        }

        s1++;
        s2++;
        len--;
    }

    return 0;
}

char *strstr(const char *haystack, const char *needle) {
    char *s = (char *) haystack;
    size_t needle_len = strlen(needle);
    while (*s != '\0') {
        if (!strncmp(s, needle, needle_len)) {
            return s;
        }

        s++;
    }

    return NULL;
}

char *strchr(const char *s, int c) {
    while (*s != c && *s != '\0') {
        s++;
    }

    return (char *) s;
}

int atoi(const char *s) {
    int x = 0;
    while ('0' <= *s && *s <= '9') {
        x = (x * 10) + (*s - '0');
        s++;
    }

    return x;
}

int memcmp(const void *p1, const void *p2, size_t len) {
    uint8_t *s1 = (uint8_t *) p1;
    uint8_t *s2 = (uint8_t *) p2;
    while (*s1 == *s2 && len > 0) {
        s1++;
        s2++;
        len--;
    }

    return (len > 0) ? *s1 - *s2 : 0;
}

void bzero(void *dst, size_t len) {
    memset(dst, 0, len);
    MEMORY_BARRIER();
}

__weak void memset(void *dst, int ch, size_t len) {
    uint8_t *d = dst;
    while (len-- > 0) {
        *d++ = ch;
    }
}

__weak void memcpy(void *dst, const void *src, size_t len) {
    uint8_t *d = dst;
    const uint8_t *s = src;
    while (len-- > 0) {
        *d++ = *s++;
    }
}

__weak void memmove(void *dst, const void *src, size_t len) {
    if ((uintptr_t) dst <= (uintptr_t) src) {
        memcpy(dst, src, len);
    } else {
        uint8_t *d = dst + len;
        const uint8_t *s = src + len;
        while (len-- > 0) {
            *--d = *--s;
        }
    }
}
