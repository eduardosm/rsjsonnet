//! A reference-count based string interner.
//!
//! # Example
//!
//! ```
//! let interner = rsjsonnet_lang::interner::StrInterner::new();
//!
//! let hello1 = interner.intern("hello");
//! let world1 = interner.intern("world");
//!
//! // Interned strings preserve their value
//! assert_eq!(hello1.value(), "hello");
//! assert_eq!(world1.value(), "world");
//!
//! // Different strings are different
//! assert_ne!(hello1, world1);
//!
//! // Interned strings are reference counted
//! let hello2 = hello1.clone(); // cheap
//! assert_eq!(hello1, hello2);
//!
//! // Interning a string again will return a reference to
//! // the existing interned string
//! let hello3 = interner.intern("hello");
//! assert_eq!(hello1, hello3);
//! ```

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

/// The string interner. See the [module level documentation](self) for more.
pub struct StrInterner {
    inner: inner::Interner<str>,
}

/// A reference counted interned string.
///
/// Cloning will increase the reference count and is equivalent to calling
/// [`StrInterner::intern`] with the same string value.
///
/// Also implements [`Eq`], [`Ord`] and [`Hash`](std::hash::Hash). Note that
/// comparison and hashing is done on the internal pointer value, not the actual
/// string value. This means that:
///
/// * It does not make sense to compare [`InternedStr`] values from different
///   [`StrInterner`] instances,
/// * [`Ord`] should not be relied to be deterministic.
///
/// ```
/// let interner = rsjsonnet_lang::interner::StrInterner::new();
///
/// let a1 = interner.intern("a");
/// let b1 = interner.intern("b");
///
/// let a2 = interner.intern("a");
/// let b2 = interner.intern("b");
///
/// // Of course, they are equal.
/// assert_eq!(a1, a2);
///
/// // "a" and "b" are not equal, but there is no guarantee on the exact ordering.
/// let cmp1 = a1.cmp(&b1);
/// assert_ne!(cmp1, std::cmp::Ordering::Equal);
///
/// // But the ordering is consistent because they came from the same interner.
/// let cmp2 = a2.cmp(&b2);
/// assert_eq!(cmp1, cmp2);
/// ```
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
    /// Creates a new [`StrInterner`].
    pub fn new() -> Self {
        Self {
            inner: inner::Interner::new(),
        }
    }

    /// Interns a string or returns a reference an already interned string.
    ///
    /// # Example
    ///
    /// ```
    /// let interner = rsjsonnet_lang::interner::StrInterner::new();
    ///
    /// let hello = interner.intern("hello");
    /// assert_eq!(hello.value(), "hello");
    /// ```
    pub fn intern(&self, value: &str) -> InternedStr {
        InternedStr {
            inner: self.inner.intern(value),
        }
    }

    /// Returns a reference to an already interned string if it exists.
    ///
    /// # Example
    ///
    /// ```
    /// let interner = rsjsonnet_lang::interner::StrInterner::new();
    ///
    /// let hello1 = interner.intern("hello");
    /// assert_eq!(hello1.value(), "hello");
    ///
    /// let hello2 = interner.get_interned("hello").unwrap();
    /// assert_eq!(hello2.value(), "hello");
    ///
    /// let world = interner.get_interned("world");
    /// assert!(world.is_none()); // Not previously interned
    /// ```
    pub fn get_interned(&self, value: &str) -> Option<InternedStr> {
        self.inner
            .get_interned(value)
            .map(|v| InternedStr { inner: v })
    }

    /// Removes all non-referenced interned strings.
    pub fn gc(&self) {
        self.inner.gc();
    }
}

impl InternedStr {
    /// Returns the underlying string value.
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

/// Similar to [`InternedStr`], but [`Ord`] and [`PartialOrd`] will compare the
/// actual string values.
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
