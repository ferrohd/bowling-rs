//! Pin-deck representation as a compact bitset.
//!
//! [`PinSet`] stores up to 16 pins in a `u16` bitmask. Pin indices are
//! zero-based: pin 0 is the head pin (or the first pin in a generic layout).

use std::{fmt, ops};

use crate::error::BowlingError;

// ---------------------------------------------------------------------------
// PinSet
// ---------------------------------------------------------------------------

/// A set of pins represented as a `u16` bitmask (up to 16 pins).
///
/// Bit *i* is `1` when pin *i* is present in the set. The type is `Copy` and
/// all operations are `const`-friendly where possible.
///
/// # Construction
///
/// ```
/// # use bowling_rs::pins::PinSet;
/// // Infallible, const-evaluable constructors:
/// let ten = PinSet::full::<10>();          // first 10 pins
/// let specific = PinSet::of([3, 5, 6, 9]); // pins by index
/// let span = PinSet::range(5, 10);         // pins 5..10
///
/// // Fallible constructor for runtime pin counts:
/// let dynamic = PinSet::try_full(10).unwrap();
///
/// // Collect from an iterator:
/// let collected: PinSet = (0..5).collect();
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PinSet(u16);

impl PinSet {
    /// The empty set (no pins).
    pub const EMPTY: Self = Self(0);

    /// Creates a full pin set with the first `N` pins standing.
    ///
    /// This is an infallible, `const`-evaluable constructor. The pin count is
    /// checked at compile time when used in a const context; exceeding 16
    /// triggers a compile-time error.
    ///
    /// ```
    /// # use bowling_rs::pins::PinSet;
    /// let ten = PinSet::full::<10>();
    /// assert_eq!(ten.count(), 10);
    /// ```
    #[inline]
    pub const fn full<const N: u8>() -> Self {
        assert!(N <= 16, "pin count exceeds maximum of 16");
        if N == 16 {
            Self(u16::MAX)
        } else {
            Self((1u16 << N) - 1)
        }
    }

    /// Creates a full pin set with the first `n` pins standing.
    ///
    /// Use [`full`](Self::full) when the count is a compile-time constant.
    /// This fallible variant is for runtime-determined pin counts (e.g. from
    /// [`Ruleset::PIN_COUNT`](crate::ruleset::Ruleset::PIN_COUNT)).
    ///
    /// # Errors
    ///
    /// Returns [`BowlingError::TooManyPins`] if `n > 16`.
    pub fn try_full(n: u8) -> Result<Self, BowlingError> {
        if n > 16 {
            return Err(BowlingError::TooManyPins(n));
        }
        if n == 16 {
            Ok(Self(u16::MAX))
        } else {
            Ok(Self((1u16 << n) - 1))
        }
    }

    /// Creates a `PinSet` containing exactly the given pin indices.
    ///
    /// ```
    /// # use bowling_rs::pins::PinSet;
    /// let ps = PinSet::of([0, 1, 2]);
    /// assert_eq!(ps.count(), 3);
    /// assert!(ps.contains(0) && ps.contains(1) && ps.contains(2));
    /// ```
    #[inline]
    pub const fn of<const N: usize>(pins: [u8; N]) -> Self {
        let mut bits = 0u16;
        let mut i = 0;
        while i < N {
            bits |= 1 << pins[i];
            i += 1;
        }
        Self(bits)
    }

    /// Creates a `PinSet` with pin indices in the half-open range
    /// `[start, end)`.
    ///
    /// ```
    /// # use bowling_rs::pins::PinSet;
    /// let ps = PinSet::range(3, 7);
    /// assert_eq!(ps, PinSet::of([3, 4, 5, 6]));
    /// ```
    #[inline]
    pub const fn range(start: u8, end: u8) -> Self {
        assert!(end <= 16, "end exceeds maximum of 16");
        assert!(start <= end, "start must not exceed end");
        if start == end {
            return Self::EMPTY;
        }
        // bits [start, end): mask with `end` bits set, minus mask with `start` bits set.
        let hi = if end == 16 {
            u16::MAX
        } else {
            (1u16 << end) - 1
        };
        let lo = if start == 16 {
            u16::MAX
        } else {
            (1u16 << start) - 1
        };
        Self(hi & !lo)
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
    #[expect(
        clippy::cast_possible_truncation,
        reason = "u16 has at most 16 bits set"
    )]
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

    /// Returns `true` if `self` and `other` share no common pins.
    #[inline]
    pub const fn is_disjoint(self, other: Self) -> bool {
        (self.0 & other.0) == 0
    }

    /// Returns `true` if every pin in `self` is also in `other`.
    #[inline]
    pub const fn is_subset_of(self, other: Self) -> bool {
        other.contains_all(self)
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
    ///
    /// Also available via the `-` operator.
    #[inline]
    pub const fn difference(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }

    /// Set intersection: pins in both `self` and `other`.
    ///
    /// Also available via the `&` operator.
    #[inline]
    pub const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// Set union: pins in either `self` or `other`.
    ///
    /// Also available via the `|` operator.
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Complement within a given pin count: the pins in `0..pin_count` that
    /// are **not** in `self`.
    ///
    /// ```
    /// # use bowling_rs::pins::PinSet;
    /// let standing = PinSet::of([0, 1, 2]);
    /// let knocked = standing.complement_within(10);
    /// assert_eq!(knocked, PinSet::range(3, 10));
    /// ```
    #[inline]
    pub fn complement_within(self, pin_count: u8) -> Self {
        Self::try_full(pin_count)
            .expect("pin_count must be <= 16")
            .difference(self)
    }

    /// Iterate over the pin indices present in the set.
    pub fn iter(self) -> PinSetIter {
        PinSetIter(self.0)
    }
}

// ---------------------------------------------------------------------------
// Operator trait impls
// ---------------------------------------------------------------------------

/// Set union: `a | b` returns pins in either set.
impl ops::BitOr for PinSet {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self {
        self.union(rhs)
    }
}

/// Set union assignment: `a |= b`.
impl ops::BitOrAssign for PinSet {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}

/// Set intersection: `a & b` returns pins in both sets.
impl ops::BitAnd for PinSet {
    type Output = Self;

    #[inline]
    fn bitand(self, rhs: Self) -> Self {
        self.intersection(rhs)
    }
}

/// Set intersection assignment: `a &= b`.
impl ops::BitAndAssign for PinSet {
    #[inline]
    fn bitand_assign(&mut self, rhs: Self) {
        *self = *self & rhs;
    }
}

/// Set difference: `a - b` returns pins in `a` that are not in `b`.
impl ops::Sub for PinSet {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self {
        self.difference(rhs)
    }
}

/// Set difference assignment: `a -= b`.
impl ops::SubAssign for PinSet {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

/// Bitwise complement: `!a` flips all 16 bits.
///
/// Note: this flips all 16 bits regardless of pin count. For a
/// complement within a specific pin count, use
/// [`complement_within`](PinSet::complement_within).
impl ops::Not for PinSet {
    type Output = Self;

    #[inline]
    fn not(self) -> Self {
        Self(!self.0)
    }
}

// ---------------------------------------------------------------------------
// FromIterator / Extend
// ---------------------------------------------------------------------------

/// Collect pin indices into a `PinSet`.
///
/// ```
/// # use bowling_rs::pins::PinSet;
/// let ps: PinSet = [0u8, 2, 4].into_iter().collect();
/// assert_eq!(ps, PinSet::of([0, 2, 4]));
/// ```
impl FromIterator<u8> for PinSet {
    fn from_iter<I: IntoIterator<Item = u8>>(iter: I) -> Self {
        let mut bits = 0u16;
        for idx in iter {
            bits |= 1 << idx;
        }
        Self(bits)
    }
}

/// Extend a `PinSet` with additional pin indices.
impl Extend<u8> for PinSet {
    fn extend<I: IntoIterator<Item = u8>>(&mut self, iter: I) {
        for idx in iter {
            self.0 |= 1 << idx;
        }
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
            #[expect(
                clippy::cast_possible_truncation,
                reason = "u16 trailing zeros fits in u8"
            )]
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
        let ps = PinSet::full::<10>();
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
        let full = PinSet::full::<10>();
        let knocked = PinSet::of([0, 1]);
        assert!(full.contains_all(knocked));
        let remaining = full - knocked;
        assert_eq!(remaining.count(), 8);
        assert!(!remaining.contains(0));
        assert!(!remaining.contains(1));
        assert!(remaining.contains(2));
    }

    #[test]
    fn iter_collects_correctly() {
        let ps = PinSet::of([1, 3]);
        let indices: Vec<u8> = ps.iter().collect();
        assert_eq!(indices, vec![1, 3]);
    }

    #[test]
    fn too_many_pins_errors() {
        assert!(PinSet::try_full(17).is_err());
    }

    #[test]
    fn full_sixteen_pins() {
        let ps = PinSet::full::<16>();
        assert_eq!(ps.count(), 16);
    }

    #[test]
    fn of_creates_correct_set() {
        let ps = PinSet::of([3, 5, 6, 9]);
        assert_eq!(ps.count(), 4);
        assert!(ps.contains(3));
        assert!(ps.contains(5));
        assert!(ps.contains(6));
        assert!(ps.contains(9));
        assert!(!ps.contains(0));
    }

    #[test]
    fn range_creates_correct_span() {
        let ps = PinSet::range(3, 7);
        assert_eq!(ps, PinSet::of([3, 4, 5, 6]));
        assert_eq!(ps.count(), 4);
    }

    #[test]
    fn range_empty_when_equal() {
        assert_eq!(PinSet::range(5, 5), PinSet::EMPTY);
    }

    #[test]
    fn from_iterator_collects() {
        let ps: PinSet = vec![0u8, 2, 4].into_iter().collect();
        assert_eq!(ps, PinSet::of([0, 2, 4]));
    }

    #[test]
    fn operators_match_named_methods() {
        let a = PinSet::of([0, 1, 2, 3]);
        let b = PinSet::of([2, 3, 4, 5]);
        assert_eq!(a | b, a.union(b));
        assert_eq!(a & b, a.intersection(b));
        assert_eq!(a - b, a.difference(b));
    }

    #[test]
    fn is_disjoint_works() {
        let a = PinSet::of([0, 1]);
        let b = PinSet::of([2, 3]);
        assert!(a.is_disjoint(b));
        assert!(!a.is_disjoint(PinSet::of([1, 2])));
    }

    #[test]
    fn is_subset_of_works() {
        let small = PinSet::of([1, 2]);
        let big = PinSet::full::<10>();
        assert!(small.is_subset_of(big));
        assert!(!big.is_subset_of(small));
    }

    #[test]
    fn complement_within_works() {
        let standing = PinSet::of([0, 1, 2]);
        assert_eq!(standing.complement_within(10), PinSet::range(3, 10));
    }

    #[test]
    fn extend_adds_pins() {
        let mut ps = PinSet::of([0, 1]);
        ps.extend([2u8, 3]);
        assert_eq!(ps, PinSet::of([0, 1, 2, 3]));
    }

    #[test]
    fn assign_operators() {
        let mut ps = PinSet::of([0, 1, 2]);
        ps |= PinSet::of([3, 4]);
        assert_eq!(ps, PinSet::range(0, 5));

        ps &= PinSet::of([1, 2, 3]);
        assert_eq!(ps, PinSet::of([1, 2, 3]));

        ps -= PinSet::of([3]);
        assert_eq!(ps, PinSet::of([1, 2]));
    }
}
