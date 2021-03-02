pub fn idle() {
    unsafe {
        println!("idle loop...");
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
