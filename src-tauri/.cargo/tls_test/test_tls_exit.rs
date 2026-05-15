use std::cell::RefCell;

thread_local! {
    static FOO: RefCell<u32> = RefCell::new(0xDEAD);
}

fn main() {
    let val = FOO.with(|f| *f.borrow());
    // Exit with low byte: 0xAD if TLS works, 0 if uninitialized
    std::process::exit((val & 0xFF) as i32);
}
