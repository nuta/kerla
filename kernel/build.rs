fn main() {
    env!("CARGO_FROM_MAKE"); // Abort if cargo is called directly -- Use make(1) instead!

    println!(
        "cargo:rerun-if-changed=arch/{arch}/{arch}.ld",
        arch = env!("ARCH")
    );
    println!(
        "cargo:rerun-if-changed=arch/{arch}/{arch}.json",
        arch = env!("ARCH")
    );
}
