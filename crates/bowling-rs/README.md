# bowling-rs

A generic, typestate-driven bowling game engine for Rust.

Handles scoring, turn rotation, fouls, split detection, and the full final-frame
fill-ball dance for ten-pin, candlepin, and duckpin bowling. The typestate
pattern means that rolling after a completed game, or building a game with zero
players, won't compile. Not "returns an error" — literally does not compile.

## Usage

```rust
use bowling_rs::prelude::*;

let game = GameBuilder::<TenPin>::new("Alice").unwrap()
    .add_player("Bob").unwrap()
    .build().unwrap();

let mut progress = Progress::AwaitingRoll(game);

loop {
    match progress {
        Progress::AwaitingRoll(g) => {
            // roll_count(n) knocks down n pins, clean delivery
            progress = g.roll_count(10).unwrap();
        }
        Progress::Complete(g) => {
            for (i, board) in g.scoreboards().iter().enumerate() {
                println!("{}: {}", g.player(i).name(), board.total);
            }
            break;
        }
    }
}
```

For finer control over which pins get knocked and whether a foul occurred, use
`Roll` directly:

```rust
use bowling_rs::prelude::*;

let game = GameBuilder::<TenPin>::new("Alice").unwrap()
    .build().unwrap();

// Knock down pins 0, 1, 2 (clean delivery)
let knocked = PinSet::of([0, 1, 2]);
let progress = game.roll(Roll::clean(knocked)).unwrap();

// A foul that physically clears all pins: frame completes, but scores 0
// let foul_strike = Roll::foul(PinSet::full::<10>());
```

## Rulesets

The engine is generic over `Ruleset`. Three variants ship out of the box:

| | Ten-Pin | Candlepin | Duckpin |
|---|---|---|---|
| Pins | 10 | 10 | 10 |
| Frames | 10 | 10 | 10 |
| Balls/frame | 2 | 3 | 3 |
| Deadwood | Cleared | Remains | Cleared |
| Strike bonus | +2 balls | +2 balls | +2 balls |
| Spare bonus | +1 ball | +1 ball | +1 ball |
| 3-ball clearance | n/a | Spare (bonus) | Flat 10 (no bonus) |
| Split detection | Yes | No | No |

The `Ruleset` trait is public. You can implement your own variant by defining
the associated constants (`PIN_COUNT`, `FRAME_COUNT`, `BALLS_PER_FRAME`,
`DEADWOOD`, `BONUS`, `ALL_DOWN_IS_SPARE`) and optionally providing a
`PinGeometry` for split detection.

## Key types

**`Game<R, P>`** is the main entry point, parameterized by a `Ruleset` and a
phase (`AwaitingRoll` or `Complete`). Methods like `roll()` and `roll_count()`
only exist on `Game<R, AwaitingRoll>`; methods like `winners()` only exist on
`Game<R, Complete>`. The `Progress` enum is what `roll()` returns — match on it
to see whether the game continues or is finished.

**`PinSet`** is a `u16` bitset representing which pins are standing (or knocked).
Ergonomic constructors: `PinSet::full::<10>()`, `PinSet::of([3, 5, 6, 9])`,
`PinSet::range(5, 10)`, or collect from an iterator. Supports set algebra via
named methods (`union`, `intersection`, `difference`) and operators (`|`, `&`,
`-`, `!`), iteration over pin indices, and pretty-printing as `{0, 3, 5}`.
All operations are `const` where possible.

**`Roll`** pairs a `PinSet` of knocked pins with a `FoulStatus`. On a foul,
the pins are physically removed from the deck but contribute 0 to the score.

**`ScoredFrame`** is the completed-frame record. It carries a `FrameKind`
(`Strike`, `Spare`, `AllDown`, `Open`), the rolls, and the base score. Bonus
scoring is computed separately by the scoring engine.

**`Scoreboard`** is what you get from `game.scoreboard(player_index)`. Each
frame is either `Resolved` (base + bonus + cumulative known) or `Pending`
(waiting on future deliveries for bonus calculation). Implements `Display` for
a text table:

```text
Frame |   1 |   2 |   3 |   4 |   5 |   6 |   7 |   8 |   9 |  10 |
Base  |  10 |  10 |  10 |  10 |  10 |  10 |  10 |  10 |  10 |  30 |
Bonus |  20 |  20 |  20 |  20 |  20 |  20 |  20 |  20 |  10 |   0 |
Cum.  |  30 |  60 |  90 | 120 | 150 | 180 | 210 | 240 | 270 | 300 |
Total: 300
```

**`PinGeometry`** holds a static adjacency graph for pin layouts. Used for
split detection — a split is when the head pin is down and the remaining pins
form disconnected components in the adjacency graph. The standard 10-pin
triangle layout is provided as `TEN_PIN_GEOMETRY`:

```rust
use bowling_rs::prelude::*;

// Classic 7-10 split (pins 6 and 9 in 0-indexed)
let standing = PinSet::of([6, 9]);
assert!(TEN_PIN_GEOMETRY.is_split(standing));

// Adjacent pins 4-5 — not a split
let standing = PinSet::of([3, 4]);
assert!(!TEN_PIN_GEOMETRY.is_split(standing));
```

## Scoring

Traditional bonus scoring: a strike adds the next 2 deliveries' scores to the
frame; a spare adds the next 1. The final frame never earns bonus lookahead but
grants fill balls (up to 3 total deliveries on a strike, 1 extra on a spare).

Fouls zero out the delivery's score contribution but the pins are still
physically removed. This matters for bonus calculation — if a foul delivery is
in the bonus window, it contributes 0.

Mid-game scoreboards work correctly. Frames without enough future rolls to
resolve bonuses are marked `Pending` rather than computed incorrectly.

## Compile-time safety

The typestate design catches entire categories of bugs at compile time:

```rust,compile_fail
use bowling_rs::prelude::*;

// This does not compile: roll() doesn't exist on Game<_, Complete>
let game: Game<TenPin, Complete> = todo!();
game.roll_count(5); // error[E0599]: method not found
```

```rust,compile_fail
use bowling_rs::prelude::*;

// This does not compile: GameBuilder::new() requires a player name
let builder = GameBuilder::<TenPin>::new(); // error: missing argument
```

These guarantees are verified by `trybuild` compile-fail tests in the test
suite.

## Testing

The test suite covers unit tests for every module, property-based tests via
`proptest` (games always terminate, scores stay in valid ranges, perfect/gutter
games are deterministic), and compile-fail tests via `trybuild`.

```sh
cargo test
```

## License

MIT
