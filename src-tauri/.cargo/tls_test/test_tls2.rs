use std::cell::RefCell;
use std::thread;

thread_local! {
    static FOO: RefCell<u32> = RefCell::new(42);
}

fn main() {
    let mut result: i32 = 0;

    // Test main thread TLS
    FOO.with(|f| {
        let v = *f.borrow();
        if v != 42 {
            result = 1;
        }
    });

    // Test TLS mutation in main thread
    FOO.with(|f| {
        *f.borrow_mut() = 99;
    });
    FOO.with(|f| {
        if *f.borrow() != 99 {
            result = 2;
        }
    });

    // Spawn a thread to test cross-thread TLS
    let handle = thread::spawn(|| {
        let mut thread_result = 0;
        FOO.with(|f| {
            if *f.borrow() != 42 {
                thread_result = 3;
            }
            *f.borrow_mut() = 77;
        });
        FOO.with(|f| {
            if *f.borrow() != 77 {
                thread_result = 4;
            }
        });
        thread_result
    });

    let thread_result = handle.join().unwrap();
    if thread_result != 0 {
        result = thread_result;
    }

    // Verify main thread still has its own TLS value
    FOO.with(|f| {
        if *f.borrow() != 99 {
            result = 5;
        }
    });

    std::process::exit(result);
}
