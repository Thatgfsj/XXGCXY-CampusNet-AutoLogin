use std::cell::RefCell;
use std::thread;

thread_local! {
    static FOO: RefCell<u32> = RefCell::new(42);
}

fn main() {
    println!("Thread local test");
    FOO.with(|f| {
        println!("FOO = {}", f.borrow());
    });

    // Spawn a thread to test TLS across threads
    let handle = thread::spawn(|| {
        FOO.with(|f| {
            *f.borrow_mut() = 99;
            println!("Thread: FOO = {}", f.borrow());
        });
    });
    handle.join().unwrap();

    FOO.with(|f| {
        println!("Main: FOO = {}", f.borrow());
    });
}
