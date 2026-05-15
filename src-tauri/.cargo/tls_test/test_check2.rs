use std::cell::RefCell;

thread_local! {
    static FOO: RefCell<u32> = RefCell::new(0xDEADBEEF);
}

fn main() {
    let val = FOO.with(|f| *f.borrow());
    // If TLS works: val = 0xDEADBEEF, exit 0
    // If TLS broken (zeroed): val = 0, exit 0 — indistinguishable
    // So also write a known value and read back
    FOO.with(|f| *f.borrow_mut() = 0xCAFEBABE);
    let val2 = FOO.with(|f| *f.borrow());
    // If TLS works: val2 = 0xCAFEBABE
    // If TLS broken (zeroed or no TLS): val2 could be anything
    // Exit with low byte of checks
    let code = if val == 0xDEADBEEF && val2 == 0xCAFEBABE { 0 } else { 1 };
    std::process::exit(code);
}
