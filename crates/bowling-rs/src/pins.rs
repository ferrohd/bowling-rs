//! Pin-deck representation as a compact bitset.
//!
//! [`PinSet`] stores up to 16 pins in a `u16` bitmask. Pin indices are
//! zero-based: pin 0 is the head pin (or the first pin in a generic layout).

use std::fmt;

use crate::error::BowlingError;

// ---------------------------------------------------------------------------
// PinSet
// ---------------------------------------------------------------------------

/// A set of pins represented as a `u16` bitmask (up to 16 pins).
///
/// Bit *i* is `1` when pin *i* is present in the set. The type is `Copy` and
/// all operations are `const`-friendly where possible.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PinSet(u16);

impl PinSet {
    /// The empty set (no pins).
    pub const EMPTY: Self = Self(0);

    /// Creates a full pin set with the first `n` pins standing.
    ///
    /// # Errors
    ///
    /// Returns [`BowlingError::TooManyPins`] if `n > 16`.
    pub fn full(n: u8) -> Result<Self, BowlingError> {
        if n > 16 {
            return Err(BowlingError::TooManyPins(n));
        }
        if n == 16 {
            Ok(Self(u16::MAX))
        } else {
            Ok(Self((1u16 << n) - 1))
        }
    }

    /// Creates a `PinSet` from a raw bitmask. No validation; caller must
    /// ensure the mask only uses bits 0..`pin_count`.
    #[inline]
    pub const fn from_raw(bits: u16) -> Self {
        Self(bits)
    }

    /// Returns the underlying bitmask.
    #[inline]
    pub const fn to_raw(self) -> u16 {
        self.0
    }

    /// Returns the number of pins in the set.
    #[inline]
    #[expect(clippy::cast_possible_truncation, reason = "u16 has at most 16 bits set")]
    pub const fn count(self) -> u8 {
        self.0.count_ones() as u8
    }

    /// Returns `true` if the set contains no pins.
    #[inline]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns `true` if pin `idx` is in the set.
    #[inline]
    pub const fn contains(self, idx: u8) -> bool {
        (self.0 >> idx) & 1 == 1
    }

    /// Adds a single pin to the set.
    #[inline]
    pub const fn insert(self, idx: u8) -> Self {
        Self(self.0 | (1 << idx))
    }

    /// Removes a single pin from the set.
    #[inline]
    pub const fn remove(self, idx: u8) -> Self {
        Self(self.0 & !(1 << idx))
    }

    /// Returns `true` if `other` is a subset of `self` (all bits in `other`
    /// are also set in `self`).
    #[inline]
    pub const fn contains_all(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Set difference: pins in `self` that are **not** in `other`.
    #[inline]
    pub const fn difference(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }

    /// Set intersection: pins in both `self` and `other`.
    #[inline]
    pub const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// Set union: pins in either `self` or `other`.
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Iterate over the pin indices present in the set.
    pub fn iter(self) -> PinSetIter {
        PinSetIter(self.0)
    }
}

impl fmt::Debug for PinSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PinSet({:#06x}, count={})", self.0, self.count())
    }
}

impl fmt::Display for PinSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        let mut first = true;
        for idx in *self {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{idx}")?;
            first = false;
        }
        write!(f, "}}")
    }
}

impl IntoIterator for PinSet {
    type Item = u8;
    type IntoIter = PinSetIter;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

// ---------------------------------------------------------------------------
// PinSetIter
// ---------------------------------------------------------------------------

/// Iterator over pin indices in a [`PinSet`].
pub struct PinSetIter(u16);

impl Iterator for PinSetIter {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        if self.0 == 0 {
            None
        } else {
            #[expect(clippy::cast_possible_truncation, reason = "u16 trailing zeros fits in u8")]
            let idx = self.0.trailing_zeros() as u8;
            self.0 &= self.0 - 1; // clear lowest set bit
            Some(idx)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let c = self.0.count_ones() as usize;
        (c, Some(c))
    }
}

impl ExactSizeIterator for PinSetIter {}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_ten_pins() {
        let ps = PinSet::full(10).unwrap();
        assert_eq!(ps.count(), 10);
        assert!(ps.contains(0));
        assert!(ps.contains(9));
        assert!(!ps.contains(10));
    }

    #[test]
    fn empty_set() {
        let ps = PinSet::EMPTY;
        assert!(ps.is_empty());
        assert_eq!(ps.count(), 0);
    }

    #[test]
    fn subset_operations() {
        let full = PinSet::full(10).unwrap();
        let knocked = PinSet::from_raw(0b0000_0011); // pins 0, 1
        assert!(full.contains_all(knocked));
        let remaining = full.difference(knocked);
        assert_eq!(remaining.count(), 8);
        assert!(!remaining.contains(0));
        assert!(!remaining.contains(1));
        assert!(remaining.contains(2));
    }

    #[test]
    fn iter_collects_correctly() {
        let ps = PinSet::from_raw(0b1010); // pins 1, 3
        let indices: Vec<u8> = ps.iter().collect();
        assert_eq!(indices, vec![1, 3]);
    }

    #[test]
    fn too_many_pins_errors() {
        assert!(PinSet::full(17).is_err());
    }

    #[test]
    fn full_sixteen_pins() {
        let ps = PinSet::full(16).unwrap();
        assert_eq!(ps.count(), 16);
    }
}
