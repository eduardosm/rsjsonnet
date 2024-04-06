#![warn(
    rust_2018_idioms,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unused_qualifications
)]
#![allow(clippy::too_many_arguments, clippy::type_complexity)]
#![forbid(unsafe_code)]

//! Library to parse and evaluate Jsonnet files.
//!
//! See the [`token`], [`lexer`], [`ast`] and [`parser`] modules for
//! Jsonnet parsing. See the [`program`] module for Jsonnet
//! evaluation.

pub mod ast;
mod float;
mod gc;
pub mod interner;
pub mod lexer;
pub mod parser;
pub mod program;
pub mod span;
pub mod token;
