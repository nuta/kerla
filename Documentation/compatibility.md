# Compatibility with Linux kernel

## Kernel Modules
Not supported.

## libc
- musl: supported
- glibc: *not* yet supported (it uses some unimplemented Linux features).

## System Calls

## The type of Implementation Status
- **Full:** All features are implemented.
- **Partial:** There're still unimplemented features are left (e.g. rarely used flags).
- **Unimplemented:** The system call is not implemented at all. It will return `ENOSYS`.

<!-- Tip: Use this VSCode plugin to edit this table: https://marketplace.visualstudio.com/items?itemName=darkriszty.markdown-table-prettify -->



| No  | Name                   | Implementation Status | Release      | Notes                                      |
|-----|------------------------|-----------------------|--------------|--------------------------------------------|
| 0   | read                   | Partially             | `v0.0.1`     |                                            |
| 1   | write                  | Partially             | `v0.0.1`     |                                            |
| 2   | open                   | Partially             | `v0.0.1`     |                                            |
| 3   | close                  | Partially             | `v0.0.1`     |                                            |
| 4   | stat                   | Partially             | `v0.0.1`     |                                            |
| 5   | fstat                  | Partially             | `v0.0.1`     |                                            |
| 6   | lstat                  | Partially             | `v0.0.1`     |                                            |
| 7   | poll                   | Partially             | `v0.0.1`     |                                            |
| 8   | lseek                  | Unimplemented         |              |                                            |
| 9   | mmap                   | Partially             | `v0.0.1`     |                                            |
| 10  | mprotect               | Unimplemented         |              |                                            |
| 11  | munmap                 | Unimplemented         |              |                                            |
| 12  | brk                    | Partially             | `v0.0.1`     |                                            |
| 13  | rt_sigaction           | Partially             | `v0.0.1`     |                                            |
| 14  | rt_sigprocmask         | Unimplemented         |              |                                            |
| 15  | rt_sigreturn           | Partially             | `v0.0.1`     |                                            |
| 16  | ioctl                  | Partially             | `v0.0.1`     |                                            |
| 17  | pread64                | Unimplemented         |              |                                            |
| 18  | pwrite64               | Unimplemented         |              |                                            |
| 19  | readv                  | Unimplemented         |              |                                            |
| 20  | writev                 | Partially             | `v0.0.1`     |                                            |
| 21  | access                 | Unimplemented         |              |                                            |
| 22  | pipe                   | Partially             | `v0.0.1`     |                                            |
| 23  | select                 | Partially             | `v0.0.1`     |                                            |
| 24  | sched_yield            | Unimplemented         |              |                                            |
| 25  | mremap                 | Unimplemented         |              |                                            |
| 26  | msync                  | Unimplemented         |              |                                            |
| 27  | mincore                | Unimplemented         |              |                                            |
| 28  | madvise                | Unimplemented         |              |                                            |
| 29  | shmget                 | Unimplemented         |              |                                            |
| 30  | shmat                  | Unimplemented         |              |                                            |
| 31  | shmctl                 | Unimplemented         |              |                                            |
| 32  | dup                    | Unimplemented         |              |                                            |
| 33  | dup2                   | Partially             | `v0.0.1`     |                                            |
| 34  | pause                  | Unimplemented         |              |                                            |
| 35  | nanosleep              | Unimplemented         |              |                                            |
| 36  | getitimer              | Unimplemented         |              |                                            |
| 37  | alarm                  | Unimplemented         |              |                                            |
| 38  | setitimer              | Unimplemented         |              |                                            |
| 39  | getpid                 | Partially             | `v0.0.1`     |                                            |
| 40  | sendfile               | Unimplemented         |              |                                            |
| 41  | socket                 | Partially             | `v0.0.1`     |                                            |
| 42  | connect                | Partially             | `v0.0.1`     |                                            |
| 43  | accept                 | Partially             | `v0.0.1`     |                                            |
| 44  | sendto                 | Partially             | `v0.0.1`     |                                            |
| 45  | recvfrom               | Partially             | `v0.0.1`     |                                            |
| 46  | sendmsg                | Unimplemented         |              |                                            |
| 47  | recvmsg                | Unimplemented         |              |                                            |
| 48  | shutdown               | Partially             | next release |                                            |
| 49  | bind                   | Partially             | `v0.0.1`     |                                            |
| 50  | listen                 | Partially             | `v0.0.1`     |                                            |
| 51  | getsockname            | Partially             | `v0.0.1`     |                                            |
| 52  | getpeername            | Partially             | `v0.0.1`     |                                            |
| 53  | socketpair             | Unimplemented         |              |                                            |
| 54  | setsockopt             | Unimplemented         |              |                                            |
| 55  | getsockopt             | Partially             | `v0.0.1`     |                                            |
| 56  | clone                  | Unimplemented         |              |                                            |
| 57  | fork                   | Partially             | `v0.0.1`     |                                            |
| 58  | vfork                  | Unimplemented         |              |                                            |
| 59  | execve                 | Partially             | `v0.0.1`     |                                            |
| 60  | exit                   | Partially             | `v0.0.1`     |                                            |
| 61  | wait4                  | Partially             | `v0.0.1`     |                                            |
| 62  | kill                   | Partially             | next release              |                                            |
| 63  | uname                  | Partially             | `v0.0.1`     |                                            |
| 64  | semget                 | Unimplemented         |              |                                            |
| 65  | semop                  | Unimplemented         |              |                                            |
| 66  | semctl                 | Unimplemented         |              |                                            |
| 67  | shmdt                  | Unimplemented         |              |                                            |
| 68  | msgget                 | Unimplemented         |              |                                            |
| 69  | msgsnd                 | Unimplemented         |              |                                            |
| 70  | msgrcv                 | Unimplemented         |              |                                            |
| 71  | msgctl                 | Unimplemented         |              |                                            |
| 72  | fcntl                  | Partially             | `v0.0.1`     |                                            |
| 73  | flock                  | Unimplemented         |              |                                            |
| 74  | fsync                  | Partially             | `v0.0.1`     |                                            |
| 75  | fdatasync              | Unimplemented         |              |                                            |
| 76  | truncate               | Unimplemented         |              |                                            |
| 77  | ftruncate              | Unimplemented         |              |                                            |
| 78  | getdents               | Unimplemented         |              |                                            |
| 79  | getcwd                 | Partially             | `v0.0.1`     |                                            |
| 80  | chdir                  | Partially             | `v0.0.1`     |                                            |
| 81  | fchdir                 | Unimplemented         |              |                                            |
| 82  | rename                 | Unimplemented         |              |                                            |
| 83  | mkdir                  | Partially             | `v0.0.1`     |                                            |
| 84  | rmdir                  | Unimplemented         |              |                                            |
| 85  | creat                  | Unimplemented         |              |                                            |
| 86  | link                   | Partially             | `v0.0.1`     |                                            |
| 87  | unlink                 | Unimplemented         |              |                                            |
| 88  | symlink                | Unimplemented         |              |                                            |
| 89  | readlink               | Partially             | `v0.0.1`     |                                            |
| 90  | chmod                  | Partially             | `v0.0.1`     |                                            |
| 91  | fchmod                 | Unimplemented         |              |                                            |
| 92  | chown                  | Partially             | `v0.0.1`     |                                            |
| 93  | fchown                 | Unimplemented         |              |                                            |
| 94  | lchown                 | Unimplemented         |              |                                            |
| 95  | umask                  | Unimplemented         |              |                                            |
| 96  | gettimeofday           | Unimplemented         |              |                                            |
| 97  | getrlimit              | Unimplemented         |              |                                            |
| 98  | getrusage              | Unimplemented         |              |                                            |
| 99  | sysinfo                | Unimplemented         |              |                                            |
| 100 | times                  | Unimplemented         |              |                                            |
| 101 | ptrace                 | Unimplemented         |              |                                            |
| 102 | getuid                 | Partially             | `v0.0.1`     |                                            |
| 103 | syslog                 | Partially             | `v0.0.2`     |                                            |
| 104 | getgid                 | Unimplemented         |              |                                            |
| 105 | setuid                 | Partially             | `v0.0.1`     |                                            |
| 106 | setgid                 | Partially             | `v0.0.1`     |                                            |
| 107 | geteuid                | Partially             | `v0.0.1`     |                                            |
| 108 | getegid                | Unimplemented         |              |                                            |
| 109 | setpgid                | Partially             | `v0.0.1`     |                                            |
| 110 | getppid                | Partially             | `v0.0.3`     | PR# ?                                      |
| 111 | getpgrp                | Unimplemented         |              |                                            |
| 112 | setsid                 | Unimplemented         |              |                                            |
| 113 | setreuid               | Unimplemented         |              |                                            |
| 114 | setregid               | Unimplemented         |              |                                            |
| 115 | getgroups              | Unimplemented         |              |                                            |
| 116 | setgroups              | Partially             | `v0.0.1`     |                                            |
| 117 | setresuid              | Unimplemented         |              |                                            |
| 118 | getresuid              | Unimplemented         |              |                                            |
| 119 | setresgid              | Unimplemented         |              |                                            |
| 120 | getresgid              | Unimplemented         |              |                                            |
| 121 | getpgid                | Partially             | `v0.0.1`     |                                            |
| 122 | setfsuid               | Unimplemented         |              |                                            |
| 123 | setfsgid               | Unimplemented         |              |                                            |
| 124 | getsid                 | Unimplemented         |              |                                            |
| 125 | capget                 | Unimplemented         |              |                                            |
| 126 | capset                 | Unimplemented         |              |                                            |
| 127 | rt_sigpending          | Unimplemented         |              |                                            |
| 128 | rt_sigtimedwait        | Unimplemented         |              |                                            |
| 129 | rt_sigqueueinfo        | Unimplemented         |              |                                            |
| 130 | rt_sigsuspend          | Unimplemented         |              |                                            |
| 131 | sigaltstack            | Unimplemented         |              |                                            |
| 132 | utime                  | Unimplemented         |              |                                            |
| 133 | mknod                  | Unimplemented         |              |                                            |
| 134 | uselib                 | Unimplemented         |              |                                            |
| 135 | personality            | Unimplemented         |              |                                            |
| 136 | ustat                  | Unimplemented         |              |                                            |
| 137 | statfs                 | Unimplemented         |              |                                            |
| 138 | fstatfs                | Unimplemented         |              |                                            |
| 139 | sysfs                  | Unimplemented         |              |                                            |
| 140 | getpriority            | Unimplemented         |              |                                            |
| 141 | setpriority            | Unimplemented         |              |                                            |
| 142 | sched_setparam         | Unimplemented         |              |                                            |
| 143 | sched_getparam         | Unimplemented         |              |                                            |
| 144 | sched_setscheduler     | Unimplemented         |              |                                            |
| 145 | sched_getscheduler     | Unimplemented         |              |                                            |
| 146 | sched_get_priority_max | Unimplemented         |              |                                            |
| 147 | sched_get_priority_min | Unimplemented         |              |                                            |
| 148 | sched_rr_get_interval  | Unimplemented         |              |                                            |
| 149 | mlock                  | Unimplemented         |              |                                            |
| 150 | munlock                | Unimplemented         |              |                                            |
| 151 | mlockall               | Unimplemented         |              |                                            |
| 152 | munlockall             | Unimplemented         |              |                                            |
| 153 | vhangup                | Unimplemented         |              |                                            |
| 154 | modify_ldt             | Unimplemented         |              |                                            |
| 155 | pivot_root             | Unimplemented         |              |                                            |
| 156 | sysctl                 | Unimplemented         |              |                                            |
| 157 | prctl                  | Unimplemented         |              |                                            |
| 158 | arch_prctl             | Partially             | `v0.0.1`     |                                            |
| 159 | adjtimex               | Unimplemented         |              |                                            |
| 160 | setrlimit              | Unimplemented         |              |                                            |
| 161 | chroot                 | Unimplemented         |              |                                            |
| 162 | sync                   | Unimplemented         |              |                                            |
| 163 | acct                   | Unimplemented         |              |                                            |
| 164 | settimeofday           | Unimplemented         |              |                                            |
| 165 | mount                  | Unimplemented         |              |                                            |
| 166 | umount2                | Unimplemented         |              |                                            |
| 167 | swapon                 | Unimplemented         |              |                                            |
| 168 | swapoff                | Unimplemented         |              |                                            |
| 169 | reboot                 | Partially             | `v0.0.3`     | Halts the system regardless of parameters. |
| 170 | sethostname            | Unimplemented         |              |                                            |
| 171 | setdomainname          | Unimplemented         |              |                                            |
| 172 | iopl                   | Unimplemented         |              |                                            |
| 173 | ioperm                 | Unimplemented         |              |                                            |
| 174 | create_module          | Unimplemented         |              |                                            |
| 175 | init_module            | Unimplemented         |              |                                            |
| 176 | delete_module          | Unimplemented         |              |                                            |
| 177 | get_kernel_syms        | Unimplemented         |              |                                            |
| 178 | query_module           | Unimplemented         |              |                                            |
| 179 | quotactl               | Unimplemented         |              |                                            |
| 180 | nfsservctl             | Unimplemented         |              |                                            |
| 181 | getpmsg                | Unimplemented         |              |                                            |
| 182 | putpmsg                | Unimplemented         |              |                                            |
| 183 | afs_syscall            | Unimplemented         |              |                                            |
| 184 | tuxcall                | Unimplemented         |              |                                            |
| 185 | security               | Unimplemented         |              |                                            |
| 186 | gettid                 | Partialy              |              | Single thread implementation               |
| 187 | readahead              | Unimplemented         |              |                                            |
| 188 | setxattr               | Unimplemented         |              |                                            |
| 189 | lsetxattr              | Unimplemented         |              |                                            |
| 190 | fsetxattr              | Unimplemented         |              |                                            |
| 191 | getxattr               | Unimplemented         |              |                                            |
| 192 | lgetxattr              | Unimplemented         |              |                                            |
| 193 | fgetxattr              | Unimplemented         |              |                                            |
| 194 | listxattr              | Unimplemented         |              |                                            |
| 195 | llistxattr             | Unimplemented         |              |                                            |
| 196 | flistxattr             | Unimplemented         |              |                                            |
| 197 | removexattr            | Unimplemented         |              |                                            |
| 198 | lremovexattr           | Unimplemented         |              |                                            |
| 199 | fremovexattr           | Unimplemented         |              |                                            |
| 200 | tkill                  | Unimplemented         |              |                                            |
| 201 | time                   | Unimplemented         |              |                                            |
| 202 | futex                  | Unimplemented         |              |                                            |
| 203 | sched_setaffinity      | Unimplemented         |              |                                            |
| 204 | sched_getaffinity      | Unimplemented         |              |                                            |
| 205 | set_thread_area        | Unimplemented         |              |                                            |
| 206 | io_setup               | Unimplemented         |              |                                            |
| 207 | io_destroy             | Unimplemented         |              |                                            |
| 208 | io_getevents           | Unimplemented         |              |                                            |
| 209 | io_submit              | Unimplemented         |              |                                            |
| 210 | io_cancel              | Unimplemented         |              |                                            |
| 211 | get_thread_area        | Unimplemented         |              |                                            |
| 212 | lookup_dcookie         | Unimplemented         |              |                                            |
| 213 | epoll_create           | Unimplemented         |              |                                            |
| 214 | epoll_ctl_old          | Unimplemented         |              |                                            |
| 215 | epoll_wait_old         | Unimplemented         |              |                                            |
| 216 | remap_file_pages       | Unimplemented         |              |                                            |
| 217 | getdents64             | Partially             | `v0.0.1`     |                                            |
| 218 | set_tid_address        | Partially             | `v0.0.1`     |                                            |
| 219 | restart_syscall        | Unimplemented         |              |                                            |
| 220 | semtimedop             | Unimplemented         |              |                                            |
| 221 | fadvise64              | Unimplemented         |              |                                            |
| 222 | timer_create           | Unimplemented         |              |                                            |
| 223 | timer_settime          | Unimplemented         |              |                                            |
| 224 | timer_gettime          | Unimplemented         |              |                                            |
| 225 | timer_getoverrun       | Unimplemented         |              |                                            |
| 226 | timer_delete           | Unimplemented         |              |                                            |
| 227 | clock_settime          | Unimplemented         |              |                                            |
| 228 | clock_gettime          | Partially             | `v0.0.1`     |                                            |
| 229 | clock_getres           | Unimplemented         |              |                                            |
| 230 | clock_nanosleep        | Unimplemented         |              |                                            |
| 231 | exit_group             | Partially             | next release |                                            |
| 232 | epoll_wait             | Unimplemented         |              |                                            |
| 233 | epoll_ctl              | Unimplemented         |              |                                            |
| 234 | tgkill                 | Unimplemented         |              |                                            |
| 235 | utimes                 | Partially             | `v0.0.1`     |                                            |
| 236 | vserver                | Unimplemented         |              |                                            |
| 237 | mbind                  | Unimplemented         |              |                                            |
| 238 | set_mempolicy          | Unimplemented         |              |                                            |
| 239 | get_mempolicy          | Unimplemented         |              |                                            |
| 240 | mq_open                | Unimplemented         |              |                                            |
| 241 | mq_unlink              | Unimplemented         |              |                                            |
| 242 | mq_timedsend           | Unimplemented         |              |                                            |
| 243 | mq_timedreceive        | Unimplemented         |              |                                            |
| 244 | mq_notify              | Unimplemented         |              |                                            |
| 245 | mq_getsetattr          | Unimplemented         |              |                                            |
| 246 | kexec_load             | Unimplemented         |              |                                            |
| 247 | waitid                 | Unimplemented         |              |                                            |
| 248 | add_key                | Unimplemented         |              |                                            |
| 249 | request_key            | Unimplemented         |              |                                            |
| 250 | keyctl                 | Unimplemented         |              |                                            |
| 251 | ioprio_set             | Unimplemented         |              |                                            |
| 252 | ioprio_get             | Unimplemented         |              |                                            |
| 253 | inotify_init           | Unimplemented         |              |                                            |
| 254 | inotify_add_watch      | Unimplemented         |              |                                            |
| 255 | inotify_rm_watch       | Unimplemented         |              |                                            |
| 256 | migrate_pages          | Unimplemented         |              |                                            |
| 257 | openat                 | Unimplemented         |              |                                            |
| 258 | mkdirat                | Unimplemented         |              |                                            |
| 259 | mknodat                | Unimplemented         |              |                                            |
| 260 | fchownat               | Unimplemented         |              |                                            |
| 261 | futimesat              | Unimplemented         |              |                                            |
| 262 | fstatat                | Unimplemented         |              |                                            |
| 263 | unlinkat               | Unimplemented         |              |                                            |
| 264 | renameat               | Unimplemented         |              |                                            |
| 265 | linkat                 | Partially             | `v0.0.1`     |                                            |
| 266 | symlinkat              | Unimplemented         |              |                                            |
| 267 | readlinkat             | Unimplemented         |              |                                            |
| 268 | fchmodat               | Unimplemented         |              |                                            |
| 269 | faccessat              | Unimplemented         |              |                                            |
| 270 | pselect                | Unimplemented         |              |                                            |
| 271 | ppoll                  | Unimplemented         |              |                                            |
| 272 | unshare                | Unimplemented         |              |                                            |
| 273 | set_robust_list        | Unimplemented         |              |                                            |
| 274 | get_robust_list        | Unimplemented         |              |                                            |
| 275 | splice                 | Unimplemented         |              |                                            |
| 276 | tee                    | Unimplemented         |              |                                            |
| 277 | sync_file_range        | Unimplemented         |              |                                            |
| 278 | vmsplice               | Unimplemented         |              |                                            |
| 279 | move_pages             | Unimplemented         |              |                                            |
| 280 | utimensat              | Unimplemented         |              |                                            |
| 281 | epoll_pwait            | Unimplemented         |              |                                            |
| 282 | signalfd               | Unimplemented         |              |                                            |
| 283 | timerfd_create         | Unimplemented         |              |                                            |
| 284 | eventfd                | Unimplemented         |              |                                            |
| 285 | fallocate              | Unimplemented         |              |                                            |
| 286 | timerfd_settime        | Unimplemented         |              |                                            |
| 287 | timerfd_gettime        | Unimplemented         |              |                                            |
| 288 | accept4                | Unimplemented         |              |                                            |
| 289 | signalfd4              | Unimplemented         |              |                                            |
| 290 | eventfd2               | Unimplemented         |              |                                            |
| 291 | epoll_create1          | Unimplemented         |              |                                            |
| 292 | dup3                   | Unimplemented         |              |                                            |
| 293 | pipe2                  | Unimplemented         |              |                                            |
| 294 | inotify_init1          | Unimplemented         |              |                                            |
| 295 | preadv                 | Unimplemented         |              |                                            |
| 296 | pwritev                | Unimplemented         |              |                                            |
| 297 | rt_tgsigqueueinfo      | Unimplemented         |              |                                            |
| 298 | perf_event_open        | Unimplemented         |              |                                            |
| 299 | recvmmsg               | Unimplemented         |              |                                            |
| 300 | fanotify_init          | Unimplemented         |              |                                            |
| 301 | fanotify_mark          | Unimplemented         |              |                                            |
| 302 | prlimit64              | Unimplemented         |              |                                            |
| 303 | name_to_handle_at      | Unimplemented         |              |                                            |
| 304 | open_by_handle_at      | Unimplemented         |              |                                            |
| 305 | clock_adjtime          | Unimplemented         |              |                                            |
| 306 | syncfs                 | Unimplemented         |              |                                            |
| 307 | sendmmsg               | Unimplemented         |              |                                            |
| 308 | setns                  | Unimplemented         |              |                                            |
| 309 | getcpu                 | Unimplemented         |              |                                            |
| 310 | process_vm_readv       | Unimplemented         |              |                                            |
| 311 | process_vm_writev      | Unimplemented         |              |                                            |
| 312 | kcmp                   | Unimplemented         |              |                                            |
| 313 | finit_module           | Unimplemented         |              |                                            |
| 314 | sched_setattr          | Unimplemented         |              |                                            |
| 315 | sched_getattr          | Unimplemented         |              |                                            |
| 316 | renameat2              | Unimplemented         |              |                                            |
| 317 | seccomp                | Unimplemented         |              |                                            |
| 318 | getrandom              | Partially             | `v0.0.1`     |                                            |
| 319 | memfd_create           | Unimplemented         |              |                                            |
| 320 | kexec_file_load        | Unimplemented         |              |                                            |
| 321 | bpf                    | Unimplemented         |              |                                            |
| 322 | execveat               | Unimplemented         |              |                                            |
| 323 | userfaultfd            | Unimplemented         |              |                                            |
| 324 | membarrier             | Unimplemented         |              |                                            |
| 325 | mlock2                 | Unimplemented         |              |                                            |
| 326 | copy_file_range        | Unimplemented         |              |                                            |
| 327 | preadv2                | Unimplemented         |              |                                            |
| 328 | pwritev2               | Unimplemented         |              |                                            |
| 329 | pkey_mprotect          | Unimplemented         |              |                                            |
| 330 | pkey_alloc             | Unimplemented         |              |                                            |
| 331 | pkey_free              | Unimplemented         |              |                                            |
| 332 | statx                  | Unimplemented         |              |                                            |
| 333 | io_pgetevents          | Unimplemented         |              |                                            |
| 334 | rseq                   | Unimplemented         |              |                                            |
| 424 | pidfd_send_signal      | Unimplemented         |              |                                            |
| 425 | io_uring_setup         | Unimplemented         |              |                                            |
| 426 | io_uring_enter         | Unimplemented         |              |                                            |
| 427 | io_uring_register      | Unimplemented         |              |                                            |
| 428 | open_tree              | Unimplemented         |              |                                            |
| 429 | move_mount             | Unimplemented         |              |                                            |
| 430 | fsopen                 | Unimplemented         |              |                                            |
| 431 | fsconfig               | Unimplemented         |              |                                            |
| 432 | fsmount                | Unimplemented         |              |                                            |
| 433 | fspick                 | Unimplemented         |              |                                            |
| 434 | pidfd_open             | Unimplemented         |              |                                            |
| 435 | clone3                 | Unimplemented         |              |                                            |
