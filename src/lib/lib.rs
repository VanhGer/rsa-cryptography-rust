#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(dead_code)]
//! This crate is a toy implementation of RSA key generation, encryption and decryption.
//!
//! It should not be used for real world applications, given it has many security flaws and shortcomings.

pub mod encoding;
pub mod error;
pub mod key;
pub mod math;
