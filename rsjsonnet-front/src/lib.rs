#![warn(
    rust_2018_idioms,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unused_qualifications
)]
#![forbid(unsafe_code)]

//! A library that provides a frontend (loading source files and printing
//! errors) for Jsonnet interpreters.
//!
//! # Example
//!
//! ```
//! let source = b"local add_one(x) = x + 1; add_one(2)";
//!
//! // Create a session, which will contain a `rsjsonnet_lang::program::Program`.
//! let mut session = rsjsonnet_front::Session::new();
//!
//! // Load a virtual (i.e., not from the file system) source file into the program
//! let Some(thunk) = session.load_virt_file("<example>", source.to_vec()) else {
//!     // `Session` printed the error for us
//!     return;
//! };
//!
//! // Evaluate it
//! let Some(value) = session.eval_value(&thunk) else {
//!     // `Session` printed the error for us
//!     return;
//! };
//!
//! assert_eq!(value.as_number(), Some(3.0));
//!
//! // Manifest the value
//! let Some(json_result) = session.manifest_json(&value, false) else {
//!     // `Session` printed the error for us
//!     return;
//! };
//!
//! assert_eq!(json_result, "3");
//! ```

mod print;
mod report;
mod session;
mod src_manager;

pub use session::Session;
