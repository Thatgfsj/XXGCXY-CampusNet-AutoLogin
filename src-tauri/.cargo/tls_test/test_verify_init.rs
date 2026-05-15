fn main() {
    // If Rust runtime initializes properly, this panics (non-zero exit)
    // If our .weak main runs instead, it returns 0
    panic!("Reached user main - runtime initialized!");
}
