//! Module containing all code for [`KeyPair`]/[`Key`] generation,
//! formatting as string, parsing from string,
//! writting and reading from files and validating.

use crate::math::mod_pow;
use num_bigint::BigUint;

mod file;
mod generation;
mod str;

/// Enum to dictate if Key is a Public or Private key.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum KeyVariant {
    /// Has a modulus, and can also have a non default exponent.
    PublicKey,
    /// Always has both and modulus and exponent.
    PrivateKey,
}

/// Represents the internal components of a Public or Private key.
///
/// In the case of a Public key with a default exponent, it is still present in the struct,
/// but can be recognized via the [`IsDefaultExponent`] trait, which is
/// implemented for [`BigUint`].
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Key {
    /// `D` or `E` part of the key.
    pub exponent: BigUint,
    /// `N` part of the key.
    pub modulus: BigUint,
    pub variant: KeyVariant,
}

/// Contains both the Public and Private keys.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct KeyPair {
    pub public_key: Key,
    pub private_key: Key,
}

impl KeyPair {
    /// Returns `true` if [`KeyPair`] is valid.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        if !(self.public_key.variant == KeyVariant::PublicKey
            && self.private_key.variant == KeyVariant::PrivateKey
            && self.public_key.modulus == self.private_key.modulus
            && self.public_key.exponent <= self.public_key.modulus)
        {
            return false;
        }
        let plain_msg = BigUint::from(12_345_678u64);
        let encoded_msg = mod_pow(
            &plain_msg,
            &self.public_key.exponent,
            &self.public_key.modulus,
        );
        let decoded_msg = mod_pow(
            &encoded_msg,
            &self.private_key.exponent,
            &self.private_key.modulus,
        );
        plain_msg == decoded_msg
    }
}

impl Key {
    #[must_use]
    pub fn is_public(&self) -> bool {
        self.variant == KeyVariant::PublicKey
    }

    #[must_use]
    pub fn is_private(&self) -> bool {
        self.variant == KeyVariant::PrivateKey
    }
}

/// Trait to determine if something is equal to the default exponent.
pub trait IsDefaultExponent {
    /// Returns if something is equal to the default exponent.
    fn is_default_exponent(&self) -> bool;
}

impl IsDefaultExponent for BigUint {
    #[must_use]
    fn is_default_exponent(&self) -> bool {
        *self == BigUint::from(Key::DEFAULT_EXPONENT)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::{Key, KeyPair, KeyVariant};
    use num_bigint::BigUint;
    use std::sync::OnceLock;

    static PAIR: OnceLock<KeyPair> = OnceLock::new();

    pub(crate) fn test_pair() -> &'static KeyPair {
        PAIR.get_or_init(|| {
            KeyPair {
                public_key: Key {
                    exponent: BigUint::from(0x1_0001u32), // default exponent
                    modulus: BigUint::from(0x9668_F701u64),
                    variant: KeyVariant::PublicKey,
                },
                private_key: Key {
                    exponent: BigUint::from(0x147B_7F71u32),
                    modulus: BigUint::from(0x9668_F701u64),
                    variant: KeyVariant::PrivateKey,
                },
            }
        })
    }
}
