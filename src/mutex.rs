#[cfg(feature = "owning_ref")]
use owning_ref::StableAddress;
#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};
#[cfg(any(debug_assertions, feature = "check"))]
use std::sync::atomic::{AtomicBool, Ordering};
use std::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    panic::{RefUnwindSafe, UnwindSafe},
};

/// A mutual exclusive lock
#[derive(Debug)]
pub struct Mutex<T: ?Sized> {
    #[cfg(any(debug_assertions, feature = "check"))]
    state: AtomicBool,
    value: UnsafeCell<T>,
}

impl<T> RefUnwindSafe for Mutex<T> where T: ?Sized {}
impl<T> UnwindSafe for Mutex<T> where T: ?Sized {}
unsafe impl<T> Send for Mutex<T> where T: ?Sized + Send {}
unsafe impl<T> Sync for Mutex<T> where T: ?Sized + Send {}

impl<T> From<T> for Mutex<T> {
    fn from(val: T) -> Self {
        Self::new(val)
    }
}

impl<T> Default for Mutex<T>
where
    T: ?Sized + Default,
{
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T> Mutex<T> {
    /// Create a new `Mutex`.
    #[inline]
    pub fn new(val: T) -> Self {
        Self {
            #[cfg(any(debug_assertions, feature = "check"))]
            state: AtomicBool::new(false),
            value: UnsafeCell::new(val),
        }
    }

    /// Consume the `Mutex`, returning the inner value.
    #[inline]
    pub fn into_inner(self) -> T {
        self.value.into_inner()
    }
}

impl<T> Mutex<T>
where
    T: ?Sized,
{
    /// Get a mutable reference of the inner value T. This is safe because we
    /// have the mutable reference of the lock.
    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        self.value.get_mut()
    }

    /// Try lock the `Mutex`, returns the mutex guard. Returns None if the
    /// `Mutex` is write locked.
    #[inline]
    pub fn try_lock<'a>(&'a self) -> Option<MutexGuard<'a, T>> {
        self.lock_exclusive().then(|| MutexGuard { lock: self })
    }

    /// Lock the `Mutex`, returns the mutex guard.
    ///
    /// # Panics
    ///
    /// If the `Mutex` is already locked, this will panic if the `check` feature
    /// is turned on.
    #[inline]
    pub fn lock<'a>(&'a self) -> MutexGuard<'a, T> {
        if !self.lock_exclusive() {
            #[cfg(any(debug_assertions, feature = "check"))]
            panic!("The lock is already write locked")
        }

        MutexGuard { lock: self }
    }

    #[inline]
    fn lock_exclusive(&self) -> bool {
        #[cfg(any(debug_assertions, feature = "check"))]
        {
            self.state
                .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
        }

        #[cfg(not(any(debug_assertions, feature = "check")))]
        true
    }

    #[inline]
    fn unlock_exclusive(&self) -> bool {
        #[cfg(any(debug_assertions, feature = "check"))]
        {
            self.state
                .compare_exchange(true, false, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
        }

        #[cfg(not(any(debug_assertions, feature = "check")))]
        true
    }
}

pub struct MutexGuard<'a, T>
where
    T: ?Sized,
{
    lock: &'a Mutex<T>,
}

impl<'a, T> Deref for MutexGuard<'a, T>
where
    T: ?Sized,
{
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        unsafe { &*self.lock.value.get() }
    }
}

impl<'a, T> DerefMut for MutexGuard<'a, T>
where
    T: ?Sized,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.value.get() }
    }
}

impl<'a, T> Drop for MutexGuard<'a, T>
where
    T: ?Sized,
{
    #[inline]
    fn drop(&mut self) {
        self.lock.unlock_exclusive();
    }
}

#[cfg(feature = "owning_ref")]
unsafe impl<'a, T: 'a> StableAddress for MutexGuard<'a, T> where T: ?Sized {}

#[cfg(feature = "serde")]
impl<T> Serialize for Mutex<T>
where
    T: Serialize + ?Sized,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.lock().serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de, T> Deserialize<'de> for Mutex<T>
where
    T: Deserialize<'de> + ?Sized,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer).map(Mutex::new)
    }
}
