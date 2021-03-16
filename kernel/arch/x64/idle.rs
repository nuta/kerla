pub fn idle() {
    unsafe {
        asm!("sti; hlt");
    }
}

pub fn halt() {
    loop {
        unsafe {
            asm!("cli; hlt");
        }
    }
}
