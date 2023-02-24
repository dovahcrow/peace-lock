peace_lock
============
[![CI Badge]][CI Page] [![Latest Version]][crates.io] [![docs.rs Badge]][docs.rs]

[CI Badge]: https://img.shields.io/github/actions/workflow/status/dovahcrow/peace-lock/test.yml?style=flat-square
[CI Page]: https://github.com/dovahcrow/peace-lock/actions/workflows/test.yml
[Latest Version]: https://img.shields.io/crates/v/peace-lock.svg?style=flat-square
[crates.io]: https://crates.io/crates/peace-lock
[docs.rs Badge]: https://img.shields.io/docsrs/peace-lock?style=flat-square
[docs.rs]: https://docs.rs/peace-lock

A Mutex/RwLock that will panic if there's contention!

peace_lock helps you sanity check your concurrent algorithm and becomes zero-cost with the check
mode disabled.

## Motivation

A lock that expects no contention seems counter-intuitive: the reason to use a
lock in the first place is to properly manage the contentions, right? Yes and no. 

Sometimes you implement a contention-free algorithm on your data structure. You 
think concurrently writing to the data structure is OK, but the compiler is
unhappy about it.

For debugging purpose, the program should shout out loudly when any contention
is detected, which implies bugs. This makes you choose `RwLock` for sanity check
during the development and unnecessarily sacrifices some performance.

Later, you rewrite the algorithm using `UnsafeCell` and
`unsafe`s to make it performant.

Can we avoid rewriting and unsafe code?

Here is a concrete example:

You want to have a hashmap that allows you change the content of the values
concurrently. You can do that because you have a scheduling algorithm ensures
that no two threads can read & write to a same value at the same time.

```rust
let map: HashMap<usize, usize> = ...;
let shared_map = Arc::new(map);

// do these two concurrently in different threads
thread::spawn(|| { *shared_map.get_mut(&1).unwrap() = 3; });
thread::spawn(|| { *shared_map.get_mut(&2).unwrap() = 4; });
```
The above code won't work because you cannot get mutable references inside the
`Arc`. But "Hey, compiler, this is safe, I guarantee!" because of your brilliant 
scheduling algorithm.

To ease the compiler, you use `RwLock`:
```rust
let map: HashMap<usize, RwLock<usize>> = ...;
let shared_map = Arc::new(map);

// do these two concurrently in different threads
thread::spawn(|| { *shared_map.get(&1).unwrap().write() = 3; });
thread::spawn(|| { *shared_map.get(&2).unwrap().write() = 4; });
```
Since you know there's no conflict, using `RwLock` unnecessarily harms the
performance.

Later you decide to do some black magic for the performance:
```rust
let map: HashMap<usize, UnsafeCell<usize>> = ...;
let shared_map = Arc::new(map);

// do these two concurrently in different threads
thread::spawn(|| { unsafe { *shared_map.get(&1).unwrap().get() = 3; } });
thread::spawn(|| { unsafe { *shared_map.get(&2).unwrap().get() = 4; } });
```
The code is running correctly, until the scheduler produces a contention.

Now you wonder: I can revert the code to `RwLock` for sanity check but there are
too much code to rewrite. Are there any simpler way for me to switch between the
performance mode and the sanity check mode?

This crate is designed for this purpose.

## Usage

peace_lock is a drop-in replacement for std/parking_lot's `Mutex` and `RwLock`.

Add `peace_lock = { version = "0.1", features = ["check"]}` to your Cargo.toml
to enable the check mode. In the check mode, calling `write` or `read` will just
panic in case of contention. This let's you know the scheduling algorithm has a
bug!


```rust
use peace_lock::{RwLock, Mutex};

let map: HashMap<usize, RwLock<usize>> = ...;
let shared_map = Arc::new(map);

// do these two concurrently in different threads
// The `write` call will panic if there's another thread writing to the value
// simultaneously.
// The panic behaviour can be disabled by feature flags so that `write` becomes
// zero-cost.
thread::spawn(|| { *shared_map.get(&1).unwrap().write() = 3; });
thread::spawn(|| { *shared_map.get(&2).unwrap().write() = 4; });

// and concurrent read is fine
thread::spawn(|| { shared_map.get(&2).unwrap().read(); });
thread::spawn(|| { shared_map.get(&2).unwrap().read(); });

// also mutex
let map: HashMap<usize, Mutex<usize>> = ...;
let shared_map = Arc::new(map);

thread::spawn(|| { *shared_map.get(&1).unwrap().lock() = 3; });
thread::spawn(|| { *shared_map.get(&2).unwrap().lock() = 4; });
```

If you want to squeeze the performance, you can disable the check by remove it
from the feature list: `peace_lock = { version = "0.1", features = [] }`. This 
will make the lock zero-cost.

## Help Wanted

I'm not that proficient in atomics. It would be super helpful if someone could help
me check if the atomic ordering is used correctly and is not too tight.
