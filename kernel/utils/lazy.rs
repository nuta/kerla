use core::ops::{Deref, DerefMut};

/// A container which can be uninitialized until the first write. As you may notice,
/// it's just a wrapper type of `Option<T>`. This type is used to make it explicit
/// that its inner value will be initialized later.
pub struct Lazy<T> {
    value: Option<T>,
}

impl<T> Lazy<T> {
    pub const fn new() -> Lazy<T> {
        Lazy { value: None }
    }

    pub fn get(&self) -> &T {
        self.value.as_ref().expect("not yet initialized")
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.value.as_mut().expect("not yet initialized")
    }

    pub fn set(&mut self, value: T) {
        self.value = Some(value);
    }
}

impl<T> Deref for Lazy<T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.get()
    }
}

impl<T> DerefMut for Lazy<T> {
    fn deref_mut(&mut self) -> &mut T {
        self.get_mut()
    }
}
