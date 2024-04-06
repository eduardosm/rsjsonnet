use std::rc::Rc;

mod inner;

impl inner::Internable for str {
    type Key = str;
    type Container = Box<str>;

    #[inline]
    fn get(this: &Box<str>) -> &str {
        this
    }

    #[inline]
    fn key(this: &Box<str>) -> &str {
        this
    }
}

impl inner::InternAs<str> for &str {
    #[inline]
    fn key(&self) -> &str {
        self
    }

    #[inline]
    fn convert(self) -> Rc<Box<str>> {
        Rc::new(self.into())
    }
}

pub struct StrInterner {
    inner: inner::Interner<str>,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InternedStr {
    inner: inner::Interned<str>,
}

impl Default for StrInterner {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl StrInterner {
    pub fn new() -> Self {
        Self {
            inner: inner::Interner::new(),
        }
    }

    pub fn intern(&self, value: &str) -> InternedStr {
        InternedStr {
            inner: self.inner.intern(value),
        }
    }

    pub fn get_interned(&self, value: &str) -> Option<InternedStr> {
        self.inner
            .get_interned(value)
            .map(|v| InternedStr { inner: v })
    }

    pub fn gc(&self) {
        self.inner.gc();
    }
}

impl InternedStr {
    #[inline]
    pub fn value(&self) -> &str {
        self.inner.value()
    }
}

impl std::fmt::Debug for InternedStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value().fmt(f)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct SortedInternedStr(pub(crate) InternedStr);

impl PartialOrd for SortedInternedStr {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }

    #[inline]
    fn lt(&self, other: &Self) -> bool {
        self.0.value() < other.0.value()
    }

    #[inline]
    fn le(&self, other: &Self) -> bool {
        self.0.value() <= other.0.value()
    }

    #[inline]
    fn gt(&self, other: &Self) -> bool {
        self.0.value() > other.0.value()
    }

    #[inline]
    fn ge(&self, other: &Self) -> bool {
        self.0.value() >= other.0.value()
    }
}

impl Ord for SortedInternedStr {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.value().cmp(other.0.value())
    }
}
