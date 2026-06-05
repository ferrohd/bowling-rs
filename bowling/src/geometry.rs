//! Pin geometry and split detection.
//!
//! A [`PinGeometry`] describes the physical adjacency of pins on the deck,
//! enabling split classification. Only rulesets that ship a geometry (e.g.
//! ten-pin) support split detection; others return `None`.

use crate::pins::PinSet;

// ---------------------------------------------------------------------------
// PinGeometry
// ---------------------------------------------------------------------------

/// Describes which pins are physically adjacent on the deck.
///
/// Adjacency is stored as a slice of `(u8, u8)` pairs where each pair means "pin
/// A is adjacent to pin B". The relation is symmetric.
#[derive(Debug, Clone)]
pub struct PinGeometry {
    /// Total number of pins in this layout.
    pin_count: u8,
    /// Adjacency pairs (unordered, symmetric).
    adjacency: &'static [(u8, u8)],
}

impl PinGeometry {
    /// Creates a new geometry from a static adjacency table.
    pub const fn new(pin_count: u8, adjacency: &'static [(u8, u8)]) -> Self {
        Self {
            pin_count,
            adjacency,
        }
    }

    /// Returns the total number of pins in this layout.
    #[inline]
    pub const fn pin_count(&self) -> u8 {
        self.pin_count
    }

    /// Returns `true` if the remaining standing pins constitute a **split**.
    ///
    /// A split occurs when:
    /// 1. The head pin (pin 0) has been knocked down.
    /// 2. At least two pins remain standing.
    /// 3. The remaining pins form two or more *disconnected components* under
    ///    the adjacency graph (i.e., there is a gap between groups of standing
    ///    pins).
    pub fn is_split(&self, standing: PinSet) -> bool {
        // Rule 1: head pin must be down
        if standing.contains(0) {
            return false;
        }

        // Rule 2: need at least 2 pins for a split
        if standing.count() < 2 {
            return false;
        }

        // Rule 3: check connectivity via BFS on standing pins
        let mut visited = PinSet::EMPTY;
        let Some(start) = standing.iter().next() else {
            return false;
        };
        let mut queue = PinSet::EMPTY.insert(start);
        visited = visited.insert(start);

        while let Some(current) = queue.iter().next() {
            queue = queue.remove(current);

            for &(a, b) in self.adjacency {
                let neighbor = if a == current {
                    b
                } else if b == current {
                    a
                } else {
                    continue;
                };
                if standing.contains(neighbor) && !visited.contains(neighbor) {
                    visited = visited.insert(neighbor);
                    queue = queue.insert(neighbor);
                }
            }
        }

        // If we haven't visited all standing pins, they're disconnected → split
        (standing - visited).count() > 0
    }
}

// ---------------------------------------------------------------------------
// Standard ten-pin geometry
// ---------------------------------------------------------------------------

/// Standard ten-pin triangle layout:
///
/// ```text
///        7  8  9  10       (row 4, indices 6..=9)
///          4  5  6         (row 3, indices 3..=5)
///            2  3          (row 2, indices 1..=2)
///              1           (row 1, index 0)
/// ```
///
/// Pins are zero-indexed internally (0 = head pin 1, etc.).
///
/// Adjacency connects each pin to its immediate neighbours in the triangle.
pub static TEN_PIN_GEOMETRY: PinGeometry = PinGeometry::new(
    10,
    &[
        // Row 1 ↔ Row 2
        (0, 1),
        (0, 2),
        // Row 2 ↔ Row 2
        (1, 2),
        // Row 2 ↔ Row 3
        (1, 3),
        (1, 4),
        (2, 4),
        (2, 5),
        // Row 3 ↔ Row 3
        (3, 4),
        (4, 5),
        // Row 3 ↔ Row 4
        (3, 6),
        (3, 7),
        (4, 7),
        (4, 8),
        (5, 8),
        (5, 9),
        // Row 4 ↔ Row 4
        (6, 7),
        (7, 8),
        (8, 9),
    ],
);

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classic_7_10_split() {
        // Pins 7 and 10 (indices 6 and 9) standing, head pin down
        let standing = PinSet::of([6, 9]);
        assert!(TEN_PIN_GEOMETRY.is_split(standing));
    }

    #[test]
    fn adjacent_pins_not_a_split() {
        // Pins 4 and 5 (indices 3 and 4) standing, head pin down
        let standing = PinSet::of([3, 4]);
        assert!(!TEN_PIN_GEOMETRY.is_split(standing));
    }

    #[test]
    fn head_pin_standing_never_split() {
        let standing = PinSet::of([0, 6, 9]);
        assert!(!TEN_PIN_GEOMETRY.is_split(standing));
    }

    #[test]
    fn single_pin_not_a_split() {
        let standing = PinSet::of([6]);
        assert!(!TEN_PIN_GEOMETRY.is_split(standing));
    }

    #[test]
    fn four_six_seven_ten_split() {
        // Pins 4, 6, 7, 10 (indices 3, 5, 6, 9).
        // Component 1: {3, 6} via edge (3, 6).
        // Component 2: {5, 9} via edge (5, 9).
        // No edges connect the two components → split.
        let standing = PinSet::of([3, 5, 6, 9]);
        assert!(TEN_PIN_GEOMETRY.is_split(standing));
    }
}
