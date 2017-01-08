extern crate libc;
extern crate x11;

#[macro_use]
pub mod util;
pub mod client;
pub mod core;
pub mod config;
pub mod loggers;
pub mod layout;
pub mod workspace;
mod atoms;
pub mod prompt;
pub mod extra;

#[allow(dead_code, non_upper_case_globals)]
mod xproto;
