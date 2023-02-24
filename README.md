peace_lock
============

A Mutex/RwLock that can only be used when there's no contention (and panics if any!).

## Motivation

A lock that expects no contention seems counter-intuitive: the reason to use a lock
in the first place is that there are contentions, right? Yes and no. 

Sometimes you implement a contention-free algorithm on your data structure. You think
having concurrent write to the data structure is OK, but the compiler is unhappy about it.
To make it worse, you must ensure the algorithm is free from bugs to be truly contention-free. This makes you choose `RwLock` by unnecessarily sacrificing some performance.
Later, to make it performant, you end up with a solution using `UnsafeCell` and making unsafe code everywhere.

Let's think about this example:

You want to change the content of the values in a hashmap concurrently. 
```rust
let map: HashMap<usize, usize> = ...;
let shared_map = Arc::new(map);

// do these two concurrently in different threads
thread::spawn(|| { *shared_map.get_mut(&1).unwrap() = 3; });
thread::spawn(|| { *shared_map.get_mut(&2).unwrap() = 4; });
```
The above code won't work because you cannot get mutable references inside the Arc.
But "Hey, compiler, this is safe, I guarantee!" because you have invented a scheduling algorithm so that
different threads will always change the values of different keys.

To circumvent compiler, you use `RwLock`:
```rust
let map: HashMap<usize, RwLock<usize>> = ...;
let shared_map = Arc::new(map);

// do these two concurrently in different threads
thread::spawn(|| { *shared_map.get(&1).unwrap().write() = 3; });
thread::spawn(|| { *shared_map.get(&2).unwrap().write() = 4; });
```
But since you know there's no conflict, using `RwLock` harms the performance.

Now you decide to do some black magic:
```rust
let map: HashMap<usize, UnsafeCell<usize>> = ...;
let shared_map = Arc::new(map);

// do these two concurrently in different threads
thread::spawn(|| { unsafe { *shared_map.get(&1).unwrap().get() = 3; } });
thread::spawn(|| { unsafe { *shared_map.get(&2).unwrap().get() = 4; } });
```
The code is running correctly, until it doesn't.
Now you wonder: am I implementing the scheduling right? Could there be a 
contention bug? I really want to use `RwLock` to sanity check but I don't want
to have the performance hit.

This crate is coming to save you.

## peace_lock
peace_lock is a drop-in replacement for std/parking_lot's `Mutex` and `RwLock`.
```rust
use peace_lock::RwLock;

let map: HashMap<usize, RwLock<usize>> = ...;
let shared_map = Arc::new(map);

// do these two concurrently in different threads
// The `write` call will panic if there's another thread is writing to the value.
// The panic behavior can be disabled by feature flags so that `write` becomes
// zero-cost.
thread::spawn(|| { *shared_map.get(&1).unwrap().write() = 3; });
thread::spawn(|| { *shared_map.get(&2).unwrap().write() = 4; });
```

You can disable the check by 
`peace_lock = { version = "0.1", default-features = false }`.

In case of contention, calling `write` or `read` will just panic, which let's 
you know the scheduling algorithm has a bug!
