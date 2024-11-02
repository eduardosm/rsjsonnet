//! Arena allocator.
//!
//! This modules provides [`Arena`], an arena allocator that can be used to
//! allocate values in heap that will all be freed at once when the [`Arena`]
//! object is dropped.

/// Arena allocator.
///
/// See the [module-level documentation](self) for more information.
pub struct Arena {
    bump: bumpalo::Bump,
}

impl Default for Arena {
    fn default() -> Self {
        Self::new()
    }
}

impl Arena {
    pub fn new() -> Self {
        Self {
            bump: bumpalo::Bump::new(),
        }
    }

    pub fn alloc<T: Copy>(&self, value: T) -> &T {
        self.bump.alloc(value)
    }

    pub fn alloc_slice<T: Copy>(&self, slice: &[T]) -> &[T] {
        self.bump.alloc_slice_copy(slice)
    }

    pub fn alloc_str(&self, value: &str) -> &str {
        self.bump.alloc_str(value)
    }
}
