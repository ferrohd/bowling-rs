//! Demonstrates split detection using the ten-pin geometry.

use bowling_rs::prelude::*;

fn main() {
    println!("=== Split Detection (Ten-Pin) ===");
    println!();
    println!("Pin layout (0-indexed):");
    println!();
    println!("        6  7  8  9       (back row)");
    println!("          3  4  5        (third row)");
    println!("            1  2         (second row)");
    println!("              0          (head pin)");
    println!();

    let cases: &[(&str, &[u8])] = &[
        ("7-10 split", &[6, 9]),
        ("4-6-7-10 (Big Four)", &[3, 5, 6, 9]),
        ("4-5 (adjacent, not a split)", &[3, 4]),
        ("7-8-10 (washout? no, head pin is down)", &[6, 7, 9]),
        ("1-2-4-10 (head pin up, never a split)", &[0, 1, 3, 9]),
        ("Single pin (not a split)", &[6]),
        ("3-6-7-10 (baby split with company)", &[2, 5, 6, 9]),
    ];

    for (name, pins) in cases {
        let standing = pins.iter().fold(PinSet::EMPTY, |s, &p| s.insert(p));
        let is_split = TEN_PIN_GEOMETRY.is_split(standing);

        let pins_display: Vec<String> = pins.iter().map(|p| (p + 1).to_string()).collect();
        let verdict = if is_split { "SPLIT" } else { "not a split" };
        println!(
            "  Pins {:<16} ({:<24}) -> {verdict}",
            pins_display.join(", "),
            name,
        );
    }
}
