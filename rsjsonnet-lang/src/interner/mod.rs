//! A reference-count based string interner.
//!
//! # Example
//!
//! ```
//! let arena = rsjsonnet_lang::arena::Arena::new();
//! let interner = rsjsonnet_lang::interner::StrInterner::new();
//!
//! let hello1 = interner.intern(&arena, "hello");
//! let world1 = interner.intern(&arena, "world");
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
//! let hello3 = interner.intern(&arena, "hello");
//! assert_eq!(hello1, hello3);
//! ```

use crate::arena::Arena;

mod inner;

impl<'a> inner::Internable<'a> for str {
    type Key = str;
    type Container = &'a str;

    #[inline]
    fn get(this: &&'a str) -> &'a str {
        this
    }

    #[inline]
    fn key<'b>(this: &'b &'a str) -> &'b str {
        this
    }
}

impl<'a> inner::InternAs<'a, str> for &str {
    #[inline]
    fn key(&self) -> &str {
        self
    }

    #[inline]
    fn convert(self, arena: &'a Arena) -> &'a &'a str {
        arena.alloc(arena.alloc_str(self))
    }
}

/// The string interner. See the [module level documentation](self) for more.
pub struct StrInterner<'a> {
    inner: inner::Interner<'a, str>,
}

/// An interned string.
///
/// Implements [`Eq`], [`Ord`] and [`Hash`](std::hash::Hash). Note that
/// comparison and hashing is done on the internal pointer value, not the actual
/// string value. This means that:
///
/// * It does not make sense to compare [`InternedStr`] values from different
///   [`StrInterner`] instances,
/// * [`Ord`] should not be relied to be deterministic.
///
/// ```
/// let arena = rsjsonnet_lang::arena::Arena::new();
/// let interner = rsjsonnet_lang::interner::StrInterner::new();
///
/// let a1 = interner.intern(&arena, "a");
/// let b1 = interner.intern(&arena, "b");
///
/// let a2 = interner.intern(&arena, "a");
/// let b2 = interner.intern(&arena, "b");
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
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InternedStr<'a> {
    inner: inner::Interned<'a, str>,
}

impl Default for StrInterner<'_> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> StrInterner<'a> {
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
    /// let arena = rsjsonnet_lang::arena::Arena::new();
    /// let interner = rsjsonnet_lang::interner::StrInterner::new();
    ///
    /// let hello = interner.intern(&arena, "hello");
    /// assert_eq!(hello.value(), "hello");
    /// ```
    pub fn intern(&self, arena: &'a Arena, value: &str) -> InternedStr<'a> {
        InternedStr {
            inner: self.inner.intern(arena, value),
        }
    }

    /// Returns a reference to an already interned string if it exists.
    ///
    /// # Example
    ///
    /// ```
    /// let arena = rsjsonnet_lang::arena::Arena::new();
    /// let interner = rsjsonnet_lang::interner::StrInterner::new();
    ///
    /// let hello1 = interner.intern(&arena, "hello");
    /// assert_eq!(hello1.value(), "hello");
    ///
    /// let hello2 = interner.get_interned("hello").unwrap();
    /// assert_eq!(hello2.value(), "hello");
    ///
    /// let world = interner.get_interned("world");
    /// assert!(world.is_none()); // Not previously interned
    /// ```
    pub fn get_interned(&self, value: &str) -> Option<InternedStr<'a>> {
        self.inner
            .get_interned(value)
            .map(|v| InternedStr { inner: v })
    }
}

impl<'a> InternedStr<'a> {
    /// Returns the underlying string value.
    #[inline]
    pub fn value(&self) -> &'a str {
        self.inner.value()
    }
}

impl std::fmt::Debug for InternedStr<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value().fmt(f)
    }
}

/// Similar to [`InternedStr`], but [`Ord`] and [`PartialOrd`] will compare the
/// actual string values.
#[derive(Copy, Clone, PartialEq, Eq)]
pub(crate) struct SortedInternedStr<'a>(pub(crate) InternedStr<'a>);

impl PartialOrd for SortedInternedStr<'_> {
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

impl Ord for SortedInternedStr<'_> {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.value().cmp(other.0.value())
    }
}
