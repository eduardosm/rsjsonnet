use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

pub(super) trait Internable {
    type Key: ?Sized + Eq + Hash;
    type Container: ?Sized;

    fn get(this: &Self::Container) -> &Self;

    fn key(this: &Self::Container) -> &Self::Key;
}

pub(super) trait InternAs<T: ?Sized + Internable> {
    fn key(&self) -> &T::Key;

    fn convert(self) -> Rc<T::Container>;
}

pub(super) struct Interner<T: ?Sized + Internable> {
    hasher: foldhash::fast::RandomState,
    items: RefCell<hashbrown::HashTable<Rc<T::Container>>>,
}

impl<T: ?Sized + Internable> Default for Interner<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T: ?Sized + Internable> Interner<T> {
    #[inline]
    pub(super) fn new() -> Self {
        Self {
            hasher: foldhash::fast::RandomState::default(),
            items: RefCell::new(hashbrown::HashTable::new()),
        }
    }

    pub(super) fn intern(&self, value: impl InternAs<T>) -> Interned<T> {
        let mut items = self.items.borrow_mut();
        match items.entry(
            Self::hash_value(value.key(), &self.hasher),
            |x| T::key(x) == value.key(),
            |x| Self::hash_value(T::key(x), &self.hasher),
        ) {
            hashbrown::hash_table::Entry::Occupied(entry) => Interned {
                item: entry.get().clone(),
            },
            hashbrown::hash_table::Entry::Vacant(entry) => {
                let rc = value.convert();
                entry.insert(rc.clone());
                Interned { item: rc }
            }
        }
    }

    pub(super) fn get_interned(&self, value: impl InternAs<T>) -> Option<Interned<T>> {
        let mut items = self.items.borrow_mut();
        match items.entry(
            Self::hash_value(value.key(), &self.hasher),
            |x| T::key(x) == value.key(),
            |x| Self::hash_value(T::key(x), &self.hasher),
        ) {
            hashbrown::hash_table::Entry::Occupied(entry) => Some(Interned {
                item: entry.get().clone(),
            }),
            hashbrown::hash_table::Entry::Vacant(_) => None,
        }
    }

    pub(super) fn gc(&self) {
        let mut items = self.items.borrow_mut();
        items.retain(|item| Rc::strong_count(item) > 1);
    }

    #[inline]
    fn hash_value(v: &T::Key, hasher: &impl std::hash::BuildHasher) -> u64 {
        hasher.hash_one(v)
    }
}

pub(super) struct Interned<T: ?Sized + Internable> {
    item: Rc<T::Container>,
}

impl<T: ?Sized + Internable> Interned<T> {
    #[inline]
    pub(crate) fn value(&self) -> &T {
        T::get(&self.item)
    }

    #[inline]
    fn ptr_as_id(&self) -> usize {
        Rc::as_ptr(&self.item).cast::<u8>() as usize
    }
}

impl<T: ?Sized + Internable> Clone for Interned<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            item: self.item.clone(),
        }
    }
}

impl<T: ?Sized + Internable> PartialEq for Interned<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.ptr_as_id() == other.ptr_as_id()
    }
}

impl<T: ?Sized + Internable> Eq for Interned<T> {}

impl<T: ?Sized + Internable> PartialOrd for Interned<T> {
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

impl<T: ?Sized + Internable> Ord for Interned<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Ord::cmp(&self.ptr_as_id(), &other.ptr_as_id())
    }
}

impl<T: ?Sized + Internable> Hash for Interned<T> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash(&self.ptr_as_id(), state);
    }
}

impl<T: ?Sized + Internable + std::fmt::Debug> std::fmt::Debug for Interned<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value().fmt(f)
    }
}
