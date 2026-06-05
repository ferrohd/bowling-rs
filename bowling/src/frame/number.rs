//! Frame number newtype.
//!
//! [`FrameNumber`] wraps a [`NonZeroU8`] to encode the invariant that frame
//! numbers are 1-indexed and never zero.

use std::fmt;
use std::num::NonZeroU8;

/// A 1-indexed frame number.
///
/// Wraps [`NonZeroU8`], making it structurally impossible to represent
/// "frame 0". Use [`FrameNumber::new`] or the `TryFrom<u8>` impl to
/// construct; both reject zero.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FrameNumber(NonZeroU8);

impl FrameNumber {
    /// Creates a frame number from a non-zero `u8`.
    ///
    /// Returns `None` if `n` is zero. For compile-time–checked literals
    /// use an inline `const` block:
    ///
    /// ```
    /// # use bowling::frame::FrameNumber;
    /// let first = const { FrameNumber::new(1).unwrap() }; // checked at compile time
    /// ```
    pub const fn new(n: u8) -> Option<Self> {
        match NonZeroU8::new(n) {
            Some(nz) => Some(Self(nz)),
            None => None,
        }
    }

    /// Returns the frame number as a plain `u8`.
    #[inline]
    pub const fn get(self) -> u8 {
        self.0.get()
    }

    /// Returns the underlying [`NonZeroU8`].
    #[inline]
    pub const fn as_non_zero(self) -> NonZeroU8 {
        self.0
    }

    /// Checked increment: returns `Some(n + 1)` or `None` on overflow.
    #[inline]
    pub const fn checked_add(self, n: u8) -> Option<Self> {
        match self.0.get().checked_add(n) {
            Some(v) => Self::new(v),
            None => None,
        }
    }
}

impl TryFrom<u8> for FrameNumber {
    type Error = FrameNumberZeroError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::new(value).ok_or(FrameNumberZeroError)
    }
}

impl From<FrameNumber> for u8 {
    fn from(f: FrameNumber) -> Self {
        f.get()
    }
}

impl fmt::Debug for FrameNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FrameNumber({})", self.0)
    }
}

impl fmt::Display for FrameNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Error returned when attempting to create a [`FrameNumber`] from zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameNumberZeroError;

impl fmt::Display for FrameNumberZeroError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "frame number must be non-zero")
    }
}

impl std::error::Error for FrameNumberZeroError {}
