//! # Categories
//!
//! Bitset-based category tagging for funding projects.
//!
//! Each [`Category`] variant maps to a single bit in a `u32` field stored on
//! [`ProjectConfig`].  Up to 32 categories can be represented with zero
//! additional storage overhead.

use soroban_sdk::contracttype;

/// A single project category, represented as a power-of-two bitmask value.
///
/// The discriminant of each variant **is** the bitmask — no conversion needed.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Category {
    Education = 1,
    Health = 2,
    Environment = 4,
    Humanitarian = 8,
    Infrastructure = 16,
    Technology = 32,
    Arts = 64,
    Research = 128,
}

impl Category {
    #[inline]
    fn bit(self) -> u32 {
        self as u32
    }
}

/// Set a category bit in `current_set`.
#[inline]
pub fn add_category(current_set: u32, category: Category) -> u32 {
    current_set | category.bit()
}

/// Clear a category bit from `current_set`.
#[inline]
pub fn remove_category(current_set: u32, category: Category) -> u32 {
    current_set & !category.bit()
}

/// Return `true` if the category bit is set in `current_set`.
#[inline]
pub fn has_category(current_set: u32, category: Category) -> bool {
    current_set & category.bit() != 0
}
