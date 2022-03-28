#include "printf.h"

static void puts(const char *s) {
    while (*s != '\0') {
        arch_printchar(*s);
        s++;
    }
}

static void print_uint(uintmax_t n, int base, char pad, int width) {
    char tmp[32];
    int i = sizeof(tmp) - 2;
    do {
        tmp[i] = "0123456789abcdef"[n % base];
        n /= base;
        width--;
        i--;
    } while (n > 0 && i > 0);

    while (width-- > 0) {
        arch_printchar(pad);
    }

    tmp[sizeof(tmp) - 1] = '\0';
    puts(&tmp[i + 1]);
}

///  The vprintf implementation.
///
///  %c   - An ASCII character.
///  %s   - An NUL-terminated ASCII string.
///  %d   - An signed integer (in decimal).
///  %u   - An unsigned integer (in decimal).
///  %x   - A hexadecimal integer.
///  %p   - An pointer value.
void vprintf(const char *fmt, va_list vargs) {
    while (*fmt) {
        if (*fmt != '%') {
            arch_printchar(*fmt++);
        } else {
            int num_len = 1;  // int
            int width = 0;
            char pad = ' ';
            bool alt = false;
            uintmax_t n;
            bool not_attr = false;
            while (*++fmt) {
                switch (*fmt) {
                    case 'l':
                        num_len++;
                        break;
                    case 'h':
                        num_len--;
                        break;
                    case '0':
                        pad = '0';
                        break;
                    case '#':
                        alt = false;
                        break;
                    case '\0':
                        puts("<invalid format>");
                        return;
                    default:
                        not_attr = true;
                }

                if (not_attr) {
                    break;
                }
            }

            if ('1' <= *fmt && *fmt <= '9') {
                width = *fmt - '0';
                fmt++;
            }

            switch (*fmt++) {
                case 'd':
                    n = (num_len == 3) ? va_arg(vargs, long long)
                                       : va_arg(vargs, int);
                    if ((intmax_t) n < 0) {
                        arch_printchar('-');
                        n = -((intmax_t) n);
                    }
                    print_uint(n, 10, pad, width);
                    break;
                case 'p': {
                    // A pointer value (%p).
                    print_uint((uintmax_t) va_arg(vargs, void *), 16, '0',
                               sizeof(vaddr_t) * 2);
                    break;
                }
                case 'x':
                    alt = true;
                    // Fallthrough.
                case 'u':
                    n = (num_len == 3) ? va_arg(vargs, unsigned long long)
                                       : va_arg(vargs, unsigned);
                    print_uint(n, alt ? 16 : 10, pad, width);
                    break;
                case 'c':
                    arch_printchar(va_arg(vargs, int));
                    break;
                case 's': {
                    char *s = va_arg(vargs, char *);
                    puts(s ? s : "(null)");
                    break;
                }
                case '%':
                    arch_printchar('%');
                    break;
                default:  // Including '\0'.
                    puts("<invalid format>");
                    return;
            }
        }
    }
}

void printf(const char *fmt, ...) {
    va_list vargs;
    va_start(vargs, fmt);
    vprintf(fmt, vargs);
    va_end(vargs);
}
