use std::cell::RefCell;
use std::thread;

thread_local! {
    static FOO: RefCell<u32> = RefCell::new(42);
}

fn main() {
    let mut code = 0;

    // Main thread
    FOO.with(|f| {
        if *f.borrow() != 42 { code = 1; }
        *f.borrow_mut() = 99;
    });

    // Spawned thread
    let h = thread::spawn(|| {
        FOO.with(|f| {
            if *f.borrow() != 42 { std::process::exit(2); }
            *f.borrow_mut() = 88;
        });
        FOO.with(|f| {
            if *f.borrow() != 88 { std::process::exit(3); }
        });
    });
    h.join().unwrap();

    // Main thread value should be independent
    FOO.with(|f| {
        if *f.borrow() != 99 { code = 4; }
    });

    if code != 0 { std::process::exit(code); }

    // Success
    std::process::exit(0);
}
