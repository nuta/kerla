use alloc::sync::Arc;
use core::any::Any;

pub trait Downcastable: Any + Send + Sync {
    fn as_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync>;
}

impl<T: Any + Send + Sync> Downcastable for T {
    fn as_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
        self
    }
}

pub fn downcast<S, T>(arc: &Arc<S>) -> Option<Arc<T>>
where
    S: Downcastable + ?Sized,
    T: Send + Sync + 'static,
{
    arc.clone().as_any().downcast::<T>().ok()
}
