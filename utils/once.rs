use core::ops::{Deref, DerefMut};

/// A value container which will be initialized extacly once. **The caller must
/// guarantee that there're no multiple threads that initialze this value simultaneously**.
pub struct Once<T> {
    inner: spin::Once<T>,
}

impl<T> Once<T> {
    pub const fn new() -> Once<T> {
        Once {
            inner: spin::Once::new(),
        }
    }

    pub fn init<F: FnOnce() -> T>(&self, f: F) {
        assert!(!self.inner.is_completed(), "already initialized");
        self.inner.call_once(f);
    }
}

impl<T> Deref for Once<T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.inner.get().expect("not yet initialized")
    }
}

impl<T> DerefMut for Once<T> {
    fn deref_mut(&mut self) -> &mut T {
        self.inner.get_mut().expect("not yet initialized")
    }
}
