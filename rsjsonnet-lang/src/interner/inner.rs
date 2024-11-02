use std::cell::RefCell;
use std::hash::{Hash, Hasher};

use crate::arena::Arena;

pub(super) trait Internable<'a>: 'a {
    type Key: ?Sized + Eq + Hash;
    type Container: 'a;

    fn get(this: &Self::Container) -> &Self;

    fn key(this: &Self::Container) -> &Self::Key;
}

pub(super) trait InternAs<'a, T: ?Sized + Internable<'a>> {
    fn key(&self) -> &T::Key;

    fn convert(self, arena: &'a Arena) -> &'a T::Container;
}

pub(super) struct Interner<'a, T: ?Sized + Internable<'a>> {
    hasher: foldhash::fast::RandomState,
    items: RefCell<hashbrown::HashTable<&'a T::Container>>,
}

impl<'a, T: ?Sized + Internable<'a>> Default for Interner<'a, T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, T: ?Sized + Internable<'a>> Interner<'a, T> {
    #[inline]
    pub(super) fn new() -> Self {
        Self {
            hasher: foldhash::fast::RandomState::default(),
            items: RefCell::new(hashbrown::HashTable::new()),
        }
    }

    pub(super) fn intern(&self, arena: &'a Arena, value: impl InternAs<'a, T>) -> Interned<'a, T> {
        let mut items = self.items.borrow_mut();
        match items.entry(
            Self::hash_value(value.key(), &self.hasher),
            |x| T::key(x) == value.key(),
            |x| Self::hash_value(T::key(x), &self.hasher),
        ) {
            hashbrown::hash_table::Entry::Occupied(entry) => Interned { item: entry.get() },
            hashbrown::hash_table::Entry::Vacant(entry) => {
                let value = value.convert(arena);
                entry.insert(value);
                Interned { item: value }
            }
        }
    }

    pub(super) fn get_interned(&self, value: impl InternAs<'a, T>) -> Option<Interned<'a, T>> {
        self.items
            .borrow()
            .find(Self::hash_value(value.key(), &self.hasher), |x| {
                T::key(x) == value.key()
            })
            .map(|&entry| Interned { item: entry })
    }

    #[inline]
    fn hash_value(v: &T::Key, hasher: &impl std::hash::BuildHasher) -> u64 {
        hasher.hash_one(v)
    }
}

pub(super) struct Interned<'a, T: ?Sized + Internable<'a>> {
    item: &'a T::Container,
}

impl<'a, T: ?Sized + Internable<'a>> Interned<'a, T> {
    #[inline]
    pub(crate) fn value(&self) -> &'a T {
        T::get(self.item)
    }

    #[inline]
    fn ptr_as_id(&self) -> usize {
        let ptr: *const T::Container = self.item;
        ptr.cast::<u8>() as usize
    }
}

impl<'a, T: ?Sized + Internable<'a>> Copy for Interned<'a, T> {}

impl<'a, T: ?Sized + Internable<'a>> Clone for Interned<'a, T> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, T: ?Sized + Internable<'a>> PartialEq for Interned<'a, T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.ptr_as_id() == other.ptr_as_id()
    }
}

impl<'a, T: ?Sized + Internable<'a>> Eq for Interned<'a, T> {}

impl<'a, T: ?Sized + Internable<'a>> PartialOrd for Interned<'a, T> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(Ord::cmp(self, other))
    }

    #[inline]
    fn lt(&self, other: &Self) -> bool {
        PartialOrd::lt(&self.ptr_as_id(), &other.ptr_as_id())
    }

    #[inline]
    fn le(&self, other: &Self) -> bool {
        PartialOrd::le(&self.ptr_as_id(), &other.ptr_as_id())
    }

    #[inline]
    fn gt(&self, other: &Self) -> bool {
        PartialOrd::gt(&self.ptr_as_id(), &other.ptr_as_id())
    }

    #[inline]
    fn ge(&self, other: &Self) -> bool {
        PartialOrd::ge(&self.ptr_as_id(), &other.ptr_as_id())
    }
}

impl<'a, T: ?Sized + Internable<'a>> Ord for Interned<'a, T> {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Ord::cmp(&self.ptr_as_id(), &other.ptr_as_id())
    }
}

impl<'a, T: ?Sized + Internable<'a>> Hash for Interned<'a, T> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash(&self.ptr_as_id(), state);
    }
}

impl<'a, T: ?Sized + Internable<'a> + std::fmt::Debug> std::fmt::Debug for Interned<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value().fmt(f)
    }
}
