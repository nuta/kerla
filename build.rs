fn main() {
    println!(
        "cargo:rerun-if-changed=src/arch/{arch}/{arch}.ld",
        arch = env!("ARCH")
    );
    println!(
        "cargo:rerun-if-changed=src/arch/{arch}/{arch}.json",
        arch = env!("ARCH")
    );
}
