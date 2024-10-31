use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

mod trace;

#[cfg(test)]
mod tests;

pub(crate) trait GcTrace {
    fn trace<'a>(&self, ctx: &mut impl GcTraceCtx<'a>)
    where
        Self: 'a;
}

pub(crate) trait GcTraceCtx<'a> {
    fn visit_obj<T: GcTrace + 'a>(&mut self, obj: &Gc<T>);
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
    #[inline]
    fn trace<'a>(&self, ctx: &mut impl GcTraceCtx<'a>)
    where
        Self: Sized + 'a,
    {
        ctx.visit_obj(self);
    }
}

impl<T: GcTrace> From<&GcView<T>> for Gc<T> {
    #[inline]
    fn from(view: &GcView<T>) -> Self {
        Self {
            inner: Rc::downgrade(&view.inner),
        }
    }
}

impl<T: GcTrace> From<&mut GcView<T>> for Gc<T> {
    #[inline]
    fn from(view: &mut GcView<T>) -> Self {
        Self {
            inner: Rc::downgrade(&view.inner),
        }
    }
}

impl<T: GcTrace> Gc<T> {
    #[inline]
    #[track_caller]
    pub(super) fn view(&self) -> GcView<T> {
        GcView {
            inner: self
                .inner
                .upgrade()
                .expect("attempted to access destroyed object"),
        }
    }
}

pub(crate) struct GcView<T: GcTrace> {
    inner: Rc<GcBox<T>>,
}

impl<T: GcTrace> Clone for GcView<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T: GcTrace> std::ops::Deref for GcView<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        &self.inner.value
    }
}

struct GcBox<T: ?Sized> {
    visits: Cell<usize>,
    mark: Cell<bool>,
    value: T,
}

trait GcTraceDyn {
    fn trace_count(&self);

    fn trace_mark<'a>(&self, ctx: &mut GcMarkCtx<'a>)
    where
        Self: 'a;
}

impl<T: GcTrace> GcTraceDyn for T {
    fn trace_count(&self) {
        self.trace(&mut GcCountCtx);
    }

    fn trace_mark<'a>(&self, ctx: &mut GcMarkCtx<'a>)
    where
        Self: 'a,
    {
        self.trace(ctx);
    }
}

pub(crate) struct GcContext<'a> {
    inner: RefCell<GcContextInner<'a>>,
}

struct GcContextInner<'a> {
    objs: Vec<Rc<GcBox<dyn GcTraceDyn + 'a>>>,
}

impl<'a> GcContext<'a> {
    #[inline]
    pub(crate) fn new() -> Self {
        Self {
            inner: RefCell::new(GcContextInner { objs: Vec::new() }),
        }
    }

    #[must_use]
    pub(crate) fn alloc<T: GcTrace + 'a>(&self, value: T) -> Gc<T> {
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
    pub(crate) fn alloc_view<T: GcTrace + 'a>(&self, value: T) -> GcView<T> {
        let mut inner = self.inner.borrow_mut();
        let obj = Rc::new(GcBox {
            visits: Cell::new(0),
            mark: Cell::new(false),
            value,
        });
        inner.objs.push(obj.clone());
        GcView { inner: obj }
    }

    #[inline]
    pub(crate) fn num_objects(&self) -> usize {
        self.inner.borrow().objs.len()
    }

    pub(crate) fn gc(&self) {
        let mut inner = self.inner.borrow_mut();
        let mut mark_ctx = GcMarkCtx { queue: Vec::new() };

        // Count (to identify roots)
        let mut known_with_view = 0;
        let mut i = 0;
        while i < inner.objs.len() {
            let obj = &inner.objs[i];
            if Rc::strong_count(obj) > 1 {
                // There is at least one `GcView`, mark directly
                if !obj.mark.get() {
                    obj.mark.set(true);
                    obj.value.trace_mark(&mut mark_ctx);
                    while let Some(sub_obj) = mark_ctx.queue.pop() {
                        debug_assert!(sub_obj.mark.get());
                        sub_obj.value.trace_mark(&mut mark_ctx);
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
                obj.value.trace_count();
                i += 1;
            } else {
                // There is at least one `Gc`, but it is already marked
                i += 1;
            }
        }

        // Mark
        for obj in inner.objs.iter() {
            if !obj.mark.get() && Rc::weak_count(obj) > obj.visits.get() {
                obj.mark.set(true);
                obj.value.trace_mark(&mut mark_ctx);
                while let Some(sub_obj) = mark_ctx.queue.pop() {
                    debug_assert!(sub_obj.mark.get());
                    sub_obj.value.trace_mark(&mut mark_ctx);
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

struct GcCountCtx;

impl<'a> GcTraceCtx<'a> for GcCountCtx {
    #[inline]
    fn visit_obj<T: GcTrace + 'a>(&mut self, obj: &Gc<T>) {
        if let Some(inner) = obj.inner.upgrade() {
            inner.visits.set(inner.visits.get() + 1);
        }
    }
}

struct GcMarkCtx<'a> {
    queue: Vec<Rc<GcBox<dyn GcTraceDyn + 'a>>>,
}

impl<'a> GcTraceCtx<'a> for GcMarkCtx<'a> {
    #[inline]
    fn visit_obj<T: GcTrace + 'a>(&mut self, obj: &Gc<T>) {
        if let Some(inner) = obj.inner.upgrade() {
            if !inner.mark.get() {
                inner.mark.set(true);
                self.queue.push(inner);
            }
        }
    }
}
