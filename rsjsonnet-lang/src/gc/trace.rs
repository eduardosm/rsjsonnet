use super::{GcTrace, GcTraceCtx};

impl<T: GcTrace> GcTrace for Option<T> {
    #[inline]
    fn trace(&self, ctx: &mut GcTraceCtx) {
        if let Some(item) = self {
            T::trace(item, ctx);
        }
    }
}

impl<T: ?Sized + GcTrace> GcTrace for std::cell::RefCell<T> {
    #[inline]
    fn trace(&self, ctx: &mut GcTraceCtx) {
        T::trace(&self.borrow(), ctx);
    }
}

impl<T: GcTrace> GcTrace for std::cell::OnceCell<T> {
    #[inline]
    fn trace(&self, ctx: &mut GcTraceCtx) {
        if let Some(item) = self.get() {
            T::trace(item, ctx);
        }
    }
}

impl<T: ?Sized + GcTrace> GcTrace for Box<T> {
    #[inline]
    fn trace(&self, ctx: &mut GcTraceCtx) {
        T::trace(self, ctx);
    }
}

impl<T: GcTrace> GcTrace for [T] {
    fn trace(&self, ctx: &mut GcTraceCtx) {
        for item in self.iter() {
            T::trace(item, ctx);
        }
    }
}

impl<T: GcTrace> GcTrace for Vec<T> {
    fn trace(&self, ctx: &mut GcTraceCtx) {
        for item in self.iter() {
            T::trace(item, ctx);
        }
    }
}
