use peace_lock::RwLock;
use std::{thread, thread::sleep, time::Duration};

#[test]
fn write_lock_unlock_lock() {
    let val = RwLock::new(1);
    thread::scope(|s| {
        s.spawn(|| {
            let lock1 = val.write();
            drop(lock1);
            sleep(Duration::from_secs(1));
        });

        s.spawn(|| {
            sleep(Duration::from_secs(1));
            let _lock2 = val.write();
        });
    });
}

#[test]
#[should_panic]
fn double_write_lock() {
    let val = RwLock::new(1);
    thread::scope(|s| {
        s.spawn(|| {
            let _lock1 = val.write();
            sleep(Duration::from_secs(1));
        });

        s.spawn(|| {
            let _lock2 = val.write();
            sleep(Duration::from_secs(1));
        });
    });
}

#[test]
#[should_panic]
fn read_write_lock_conflict1() {
    let val = RwLock::new(1);
    thread::scope(|s| {
        s.spawn(|| {
            let _lock1 = val.write();
            sleep(Duration::from_secs(2));
        });

        s.spawn(|| {
            let _lock2 = val.read();
            sleep(Duration::from_secs(1));
        });
    });
}

#[test]
#[should_panic]
fn read_write_lock_conflict2() {
    let val = RwLock::new(1);
    thread::scope(|s| {
        s.spawn(|| {
            let _lock1 = val.read();
            sleep(Duration::from_secs(2));
        });

        s.spawn(|| {
            let _lock2 = val.write();
            sleep(Duration::from_secs(1));
        });
    });
}

#[test]
fn multiple_read() {
    let val = RwLock::new(1);
    thread::scope(|s| {
        s.spawn(|| {
            let _lock1 = val.read();
            sleep(Duration::from_secs(2));
        });

        s.spawn(|| {
            let _lock2 = val.read();
            sleep(Duration::from_secs(1));
        });
    });
}
