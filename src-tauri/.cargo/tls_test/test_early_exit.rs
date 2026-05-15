// Test: exit immediately in mainCRTStartup to see if we even reach it
// The custom CRT startup is already in crt_stubs.obj, so we just need
// a test that will show whether we crash before or after entry point.

use std::cell::RefCell;

thread_local! {
    static FOO: RefCell<u32> = RefCell::new(42);
}

fn main() {
    // If we reach here, the entry point works.
    // The crash is either in the OS loader (before entry) or in Rust init.
    std::process::exit(77);
}
