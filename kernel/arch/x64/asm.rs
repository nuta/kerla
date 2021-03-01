pub unsafe fn out8(port: u16, value: u8) {
    asm!(
        "out dx, al",
        in("dx") port,
        in("al") value,
    );
}

pub unsafe fn out16(port: u16, value: u16) {
    asm!(
        "out dx, ax",
        in("dx") port,
        in("ax") value,
    );
}

pub unsafe fn in8(port: u16) -> u8 {
    let mut value;
    asm!(
        "in al, dx",
        in("dx") port,
        out("al") value,
    );
    value
}
