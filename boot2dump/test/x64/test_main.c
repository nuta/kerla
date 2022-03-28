extern char __boot2dump[];
typedef void (*boot2dump_entry_t)(const char *path, unsigned long long path_len,
                                  unsigned char *buf,
                                  unsigned long long buf_len);

void test_main(void) {
    unsigned char buf[] = "Hello World from test_main!";
    boot2dump_entry_t entry = (boot2dump_entry_t) __boot2dump;
    entry("boot.dump!!!!!!!!", 9, buf, sizeof(buf));
    for (;;) {}
}
