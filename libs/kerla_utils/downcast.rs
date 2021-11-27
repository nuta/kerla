use alloc::sync::Arc;
use core::any::Any;

pub trait Downcastable: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
}

impl<T: Any + Send + Sync> Downcastable for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub fn downcast<S, T>(arc: &Arc<S>) -> Option<&Arc<T>>
where
    S: Downcastable + ?Sized,
    T: Send + Sync + 'static,
{
    arc.as_any().downcast_ref::<Arc<T>>()
}
