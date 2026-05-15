use std::cell::RefCell;

thread_local! {
    static FOO: RefCell<u32> = RefCell::new(42);
}

fn main() {
    let val = FOO.with(|f| *f.borrow());
    // If TLS works, val should be 42. If not, val is 0 (zeroed memory).
    // Return 0 if val is 42, otherwise return val.
    if val == 42 {
        std::process::exit(0);
    } else {
        std::process::exit(val as i32);
    }
}
