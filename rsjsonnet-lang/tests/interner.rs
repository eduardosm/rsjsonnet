#![warn(
    rust_2018_idioms,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unused_qualifications
)]
#![forbid(unsafe_code)]

use rsjsonnet_lang::interner::StrInterner;

#[test]
fn test_intern_str() {
    let interner = StrInterner::new();

    let hello_1 = interner.intern("hello");
    let world_1 = interner.intern("world");
    assert_ne!(hello_1, world_1);
    assert_eq!(hello_1.value(), "hello");
    assert_eq!(world_1.value(), "world");

    let hello_2 = interner.intern("hello");
    assert_eq!(hello_1, hello_2);
    assert_ne!(hello_2, world_1);

    let world_2 = interner.intern("world");
    assert_eq!(world_1, world_2);
    assert_ne!(hello_1, world_2);
    assert_ne!(hello_2, world_2);
}

#[test]
fn test_get_interned_str() {
    let interner = StrInterner::new();

    let hello_1 = interner.get_interned("hello");
    assert!(hello_1.is_none());

    let hello_2 = interner.intern("hello");
    let hello_3 = interner.get_interned("hello");
    assert_eq!(hello_3, Some(hello_2));
}
