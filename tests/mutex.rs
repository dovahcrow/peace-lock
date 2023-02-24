use peace_lock::Mutex;
use std::{thread, thread::sleep, time::Duration};

#[test]
fn lock_unlock_lock() {
    let val = Mutex::new(1);
    thread::scope(|s| {
        s.spawn(|| {
            let lock1 = val.lock();
            drop(lock1);
            sleep(Duration::from_secs(1));
        });

        s.spawn(|| {
            sleep(Duration::from_secs(1));
            let _lock2 = val.lock();
        });
    });
}

#[test]
#[should_panic]
fn double_lock() {
    let val = Mutex::new(1);
    thread::scope(|s| {
        s.spawn(|| {
            let _lock1 = val.lock();
            sleep(Duration::from_secs(1));
        });

        s.spawn(|| {
            let _lock2 = val.lock();
            sleep(Duration::from_secs(1));
        });
    });
}
