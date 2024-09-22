#
#  Busybox
#
FROM ubuntu:20.04 as busybox
RUN apt-get update && apt-get install -qy build-essential curl sed
RUN apt-get install -qy musl-tools
RUN curl -fsSL --output tarball.tar.bz2 "https://busybox.net/downloads/busybox-1.31.1.tar.bz2"
RUN mkdir /build && tar xf tarball.tar.bz2 --strip-components=1 -C /build
WORKDIR /build
ADD busybox/config /build/.config
RUN yes n | make oldconfig \
    && ln -s /usr/bin/ar /usr/bin/musl-ar \
    && ln -s /usr/bin/strip /usr/bin/musl-strip \
    && make -j$(nproc)

#
#  Curl
#
FROM ubuntu:20.04 as curl
RUN apt-get update && apt-get install -qy build-essential curl sed
RUN apt-get install -qy musl-tools
RUN curl -fsSL --output tarball.tar.xz "https://curl.se/download/curl-7.76.1.tar.xz"
RUN mkdir /build && tar xf tarball.tar.xz --strip-components=1 -C /build
WORKDIR /build
RUN ./configure CC=musl-gcc --disable-shared --disable-pthreads --disable-threaded-resolver --disable-rtsp --disable-alt-svc --disable-libcurl-option --disable-telnet --disable-gopher --disable-dict --disable-file --disable-ftp --disable-tftp --disable-imap --disable-pop3 --disable-smtp --disable-mqtt --disable-unix-sockets
RUN make -j$(nproc) curl_LDFLAGS=-all-static

#
#  Dropbear SSH server
#
FROM ubuntu:20.04 as dropbear
RUN apt-get update && apt-get install -qy build-essential curl sed
RUN apt-get install -qy musl-tools
RUN curl -fsSL --output tarball.tar.bz2 "https://matt.ucc.asn.au/dropbear/releases/dropbear-2020.81.tar.bz2"
RUN mkdir /build && tar xf tarball.tar.bz2 --strip-components=1 -C /build
ADD dropbear/accept-empty-password-root-login.patch /build/accept-empty-password-root-login.patch
ADD dropbear/localoptions.h /build/localoptions.h
WORKDIR /build
RUN sh -c 'patch --ignore-whitespace -p1 < accept-empty-password-root-login.patch'
RUN ./configure CC=musl-gcc --enable-static --disable-largefile --disable-zlib --disable-syslog --disable-wtmp --disable-wtmpx --disable-utmp --disable-utmpx --disable-loginfunc
RUN make -j$(nproc)

#
#  Initramfs
#
FROM busybox as empty_dir
RUN mkdir /empty_dir

FROM scratch

# Directories to be populated by "busybox --install -s".
COPY --from=empty_dir /empty_dir /bin
COPY --from=empty_dir /empty_dir /sbin
COPY --from=empty_dir /empty_dir /usr/bin
COPY --from=empty_dir /empty_dir /usr/sbin

COPY --from=busybox  /build/busybox_unstripped /bin/busybox
COPY --from=dropbear /build/dropbear /build/dropbearkey /bin
COPY --from=curl     /build/src/curl /bin

RUN ["/bin/busybox", "--install", "-s"]
RUN mkdir -p /etc /dev /tmp /var/www/html
ADD etc/banner /etc
ADD etc/group /etc
ADD etc/passwd /etc
ADD etc/profile /etc
ADD integration_tests /integration_tests
ADD var/www/html/index.html /var/www/html/index.html

CMD ["/bin/sh"]
