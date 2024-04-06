use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

mod trace;

#[cfg(test)]
mod tests;

pub(crate) trait GcTrace: 'static {
    fn trace(&self, ctx: &mut GcTraceCtx);
}

pub(crate) struct GcTraceCtx {
    marking: bool,
    queue: Vec<Rc<GcBox<dyn GcTrace>>>,
}

pub(crate) struct Gc<T: GcTrace> {
    inner: Weak<GcBox<T>>,
}

impl<T: GcTrace> Clone for Gc<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T: GcTrace> GcTrace for Gc<T> {
    fn trace(&self, ctx: &mut GcTraceCtx) {
        if let Some(inner) = self.inner.upgrade() {
            if !ctx.marking {
                // Count
                inner.visits.set(inner.visits.get() + 1);
            } else if !inner.mark.get() {
                // Mark
                inner.mark.set(true);
                ctx.queue.push(inner);
            }
        }
    }
}

impl<T: GcTrace> From<&GcView<T>> for Gc<T> {
    #[inline]
    fn from(view: &GcView<T>) -> Self {
        Self {
            inner: Rc::downgrade(&view.inner_),
        }
    }
}

impl<T: GcTrace> From<&mut GcView<T>> for Gc<T> {
    #[inline]
    fn from(view: &mut GcView<T>) -> Self {
        Self {
            inner: Rc::downgrade(&view.inner_),
        }
    }
}

impl<T: GcTrace> Gc<T> {
    #[inline]
    #[track_caller]
    pub(super) fn view(&self) -> GcView<T> {
        GcView {
            inner_: self
                .inner
                .upgrade()
                .expect("attempted to access destroyed object"),
        }
    }
}

pub(crate) struct GcView<T: GcTrace> {
    inner_: Rc<GcBox<T>>,
}

impl<T: GcTrace> Clone for GcView<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            inner_: self.inner_.clone(),
        }
    }
}

impl<T: GcTrace> std::ops::Deref for GcView<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        &self.inner_.value
    }
}

struct GcBox<T: ?Sized + GcTrace> {
    visits: Cell<usize>,
    mark: Cell<bool>,
    value: T,
}

pub(crate) struct GcContext {
    inner: RefCell<GcContextInner>,
}

struct GcContextInner {
    objs: Vec<Rc<GcBox<dyn GcTrace>>>,
}

impl GcContext {
    #[inline]
    pub(crate) fn new() -> Self {
        Self {
            inner: RefCell::new(GcContextInner { objs: Vec::new() }),
        }
    }

    #[must_use]
    pub(crate) fn alloc<T: GcTrace>(&self, value: T) -> Gc<T> {
        let mut inner = self.inner.borrow_mut();
        let obj = Rc::new(GcBox {
            visits: Cell::new(0),
            mark: Cell::new(false),
            value,
        });
        let weak = Rc::downgrade(&obj);
        inner.objs.push(obj);
        Gc { inner: weak }
    }

    #[must_use]
    pub(crate) fn alloc_view<T: GcTrace>(&self, value: T) -> GcView<T> {
        let mut inner = self.inner.borrow_mut();
        let obj = Rc::new(GcBox {
            visits: Cell::new(0),
            mark: Cell::new(false),
            value,
        });
        inner.objs.push(obj.clone());
        GcView { inner_: obj }
    }

    #[inline]
    pub(crate) fn num_objects(&self) -> usize {
        self.inner.borrow().objs.len()
    }

    pub(crate) fn gc(&self) {
        let mut inner = self.inner.borrow_mut();
        let mut trace_ctx = GcTraceCtx {
            marking: false,
            queue: Vec::new(),
        };

        // Count (to identify roots)
        let mut known_with_view = 0;
        let mut i = 0;
        while i < inner.objs.len() {
            let obj = &inner.objs[i];
            if Rc::strong_count(obj) > 1 {
                // There is at least one `GcView`, mark directly
                if !obj.mark.get() {
                    trace_ctx.marking = true;
                    obj.mark.set(true);
                    obj.value.trace(&mut trace_ctx);
                    while let Some(sub_obj) = trace_ctx.queue.pop() {
                        debug_assert!(sub_obj.mark.get());
                        sub_obj.value.trace(&mut trace_ctx);
                    }
                }
                if i > known_with_view {
                    // Move objects with `GcView` to the beginning to increase
                    // the chance of marking them first during the next GC.
                    inner.objs.swap(i, known_with_view);
                    known_with_view += 1;
                }
                i += 1;
            } else if Rc::weak_count(obj) == 0 {
                // There is not any `Gc` or `GcView`, destroy directly.
                inner.objs.swap_remove(i);
            } else if !obj.mark.get() {
                // There is at least one `Gc`, count
                trace_ctx.marking = false;
                obj.value.trace(&mut trace_ctx);
                i += 1;
            } else {
                // There is at least one `Gc`, but it is already marked
                i += 1;
            }
        }

        // Mark
        trace_ctx.marking = true;
        for obj in inner.objs.iter() {
            if !obj.mark.get() && Rc::weak_count(obj) > obj.visits.get() {
                obj.mark.set(true);
                obj.value.trace(&mut trace_ctx);
                while let Some(sub_obj) = trace_ctx.queue.pop() {
                    debug_assert!(sub_obj.mark.get());
                    sub_obj.value.trace(&mut trace_ctx);
                }
            }
        }

        // Sweep
        let mut i = 0;
        while i < inner.objs.len() {
            let obj = &inner.objs[i];
            if obj.mark.get() {
                obj.visits.set(0);
                obj.mark.set(false);
                i += 1;
            } else {
                inner.objs.swap_remove(i);
            }
        }
    }
}
