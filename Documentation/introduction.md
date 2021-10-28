# Introduction
Kerla is a monolithic operating system kernel from scratch in Rust which aims to be compatible with the Linux ABI, that is, runs Linux binaries without any modifications.

You can play with Kerla over ssh. Your login is not visible from others (except me): we automatically launch a dedicated microVM on Firecracker for each TCP connection.

```
$ ssh root@kerla-demo.seiya.me
```

- [GitHub](https://github.com/nuta/kerla)








