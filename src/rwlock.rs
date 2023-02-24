#[cfg(feature = "owning_ref")]
use owning_ref::StableAddress;
#[cfg(feature = "check")]
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    panic::{RefUnwindSafe, UnwindSafe},
};

// Locking bits are copied from [parking_lot](https://github.com/Amanieu/parking_lot).
// If the reader count is zero: a writer is currently holding an exclusive lock.
// Otherwise: a writer is waiting for the remaining readers to exit the lock.
#[cfg(feature = "check")]
const WRITER_BIT: usize = 0b1000;
// Base unit for counting readers.
#[cfg(feature = "check")]
const ONE_READER: usize = 0b10000;

/// A read-write lock
#[derive(Debug)]
pub struct RwLock<T: ?Sized> {
    #[cfg(feature = "check")]
    state: AtomicUsize,
    value: UnsafeCell<T>,
}

impl<T> RefUnwindSafe for RwLock<T> where T: ?Sized {}
impl<T> UnwindSafe for RwLock<T> where T: ?Sized {}
unsafe impl<T> Send for RwLock<T> where T: ?Sized + Send {}
unsafe impl<T> Sync for RwLock<T> where T: ?Sized + Send + Sync {}

impl<T> From<T> for RwLock<T> {
    fn from(val: T) -> Self {
        Self::new(val)
    }
}

impl<T> Default for RwLock<T>
where
    T: ?Sized + Default,
{
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T> RwLock<T> {
    /// Create a new `RwLock`.
    pub const fn new(val: T) -> Self {
        Self {
            value: UnsafeCell::new(val),
            #[cfg(feature = "check")]
            state: AtomicUsize::new(0),
        }
    }

    /// Consume the `RwLock`, returning the inner value.
    pub fn into_inner(self) -> T {
        self.value.into_inner()
    }
}

impl<T> RwLock<T>
where
    T: ?Sized,
{
    /// Get a mutable reference of the inner value T. This is safe because we
    /// have the mutable reference of the lock.
    pub fn get_mut(&mut self) -> &mut T {
        self.value.get_mut()
    }

    /// Try write lock the `RwLock`, returns the write guard. Returns None if the
    /// `RwLock` is write locked.
    pub fn try_write<'a>(&'a self) -> Option<RwLockWriteGuard<'a, T>> {
        self.lock_exclusive()
            .then(|| RwLockWriteGuard { lock: self })
    }

    /// Write lock the `RwLock`, returns the write guard.
    ///
    /// # Panics
    ///
    /// If the `RwLock` is already write locked, this will panic if the `check`
    /// feature is turned on.
    pub fn write<'a>(&'a self) -> RwLockWriteGuard<'a, T> {
        if !self.lock_exclusive() {
            #[cfg(feature = "check")]
            panic!("The lock is already write locked")
        }

        RwLockWriteGuard { lock: self }
    }

    /// Try read lock the `RwLock`, returns the read guard. Returns None if the
    /// `RwLock` is write locked.
    pub fn try_read<'a>(&'a self) -> Option<RwLockReadGuard<'a, T>> {
        self.lock_shared().then(|| RwLockReadGuard { lock: self })
    }

    /// Read lock the `RwLock`, returns the read guard.
    ///
    /// # Panics
    ///
    /// If the `RwLock` is already write locked, this will panic if the check feature
    /// is turned on.
    pub fn read<'a>(&'a self) -> RwLockReadGuard<'a, T> {
        if !self.lock_shared() {
            #[cfg(feature = "check")]
            panic!("The lock is already write locked")
        }

        RwLockReadGuard { lock: self }
    }

    fn lock_exclusive(&self) -> bool {
        #[cfg(feature = "check")]
        {
            self.state
                .compare_exchange(0, WRITER_BIT, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
        }

        #[cfg(not(feature = "check"))]
        true
    }

    fn unlock_exclusive(&self) -> bool {
        #[cfg(feature = "check")]
        {
            self.state
                .compare_exchange(WRITER_BIT, 0, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
        }

        #[cfg(not(feature = "check"))]
        true
    }

    fn lock_shared(&self) -> bool {
        #[cfg(feature = "check")]
        loop {
            let state = self.state.load(Ordering::Relaxed);
            if state & WRITER_BIT != 0 {
                // is write locked
                return false;
            }

            if self
                .state
                .compare_exchange(
                    state,
                    state.checked_add(ONE_READER).expect("too many readers"),
                    Ordering::Acquire,
                    Ordering::Relaxed,
                )
                .is_ok()
            {
                break;
            }
        }

        true
    }

    fn unlock_shared(&self) {
        #[cfg(feature = "check")]
        self.state.fetch_sub(ONE_READER, Ordering::Release);
    }
}

pub struct RwLockWriteGuard<'a, T>
where
    T: ?Sized,
{
    lock: &'a RwLock<T>,
}

impl<'a, T> Deref for RwLockWriteGuard<'a, T>
where
    T: ?Sized,
{
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.lock.value.get() }
    }
}

impl<'a, T> DerefMut for RwLockWriteGuard<'a, T>
where
    T: ?Sized,
{
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.value.get() }
    }
}

impl<'a, T> Drop for RwLockWriteGuard<'a, T>
where
    T: ?Sized,
{
    fn drop(&mut self) {
        self.lock.unlock_exclusive();
    }
}

pub struct RwLockReadGuard<'a, T>
where
    T: ?Sized,
{
    lock: &'a RwLock<T>,
}

impl<'a, T> Deref for RwLockReadGuard<'a, T>
where
    T: ?Sized,
{
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.lock.value.get() }
    }
}

impl<'a, T> Drop for RwLockReadGuard<'a, T>
where
    T: ?Sized,
{
    fn drop(&mut self) {
        self.lock.unlock_shared();
    }
}

#[cfg(feature = "owning_ref")]
unsafe impl<'a, T: 'a> StableAddress for RwLockReadGuard<'a, T> where T: ?Sized {}
#[cfg(feature = "owning_ref")]
unsafe impl<'a, T: 'a> StableAddress for RwLockWriteGuard<'a, T> where T: ?Sized {}
