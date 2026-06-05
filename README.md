# bowling

![docs.rs](https://img.shields.io/docsrs/bowling)
![Crates.io](https://img.shields.io/crates/v/bowling)
![License: GPL-3.0-or-later](https://img.shields.io/badge/license-GPL--3.0--or--later-blue)
![Crates.io Total Downloads](https://img.shields.io/crates/d/bowling)
![GitHub Repo stars](https://img.shields.io/github/stars/ferrohd/bowling)

A bowling game engine in Rust. It handles scoring, frame tracking, multiplayer turn rotation, and split detection for ten-pin, candlepin, and duckpin bowling.

The interesting bit is that the game is modelled as a typestate machine. Rolling on a finished game or building a game with no players are compile errors, not runtime ones. Frames work the same way internally: each delivery consumes the current frame state and produces the next one, so invalid transitions can't happen.

## Usage

```rust
use bowling::prelude::*;

let alice = Player::new("Alice").unwrap();
let bob = Player::new("Bob").unwrap();
let mut progress = GameBuilder::<TenPin>::new(alice)
    .add_player(bob)
    .build();

loop {
    match progress {
        Progress::AwaitingRoll(g) => {
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

`roll_count(n)` picks `n` pins for you. If you need to control which specific pins get knocked down, or record a foul, use `roll()` with a `Roll` directly:

```rust
// specific pins
let delivery = Roll::clean(PinSet::of([0, 1, 2]));
let progress = game.roll(delivery).unwrap();

// foul: pins physically fell, but score is zero
let foul = Roll::foul(PinSet::of([0, 1, 2]));
```

Turn rotation between players is handled automatically. After each player finishes a frame, the next one bowls the same frame, and so on.

## Supported rulesets

| | Ten-pin | Candlepin | Duckpin |
|---|---|---|---|
| Balls per frame | 2 | 3 | 3 |
| Deadwood | Cleared | Remains | Cleared |
| 3-ball clearance | n/a | Spare (bonus) | AllDown (no bonus) |
| Split detection | Yes | No | No |

All three use 10 pins, 10 frames, and the usual bonus scoring (strike = next 2 balls, spare = next 1 ball). The `Ruleset` trait is open, so you can implement your own variant if you need different pin counts, frame counts, deadwood rules, etc.

## Pins

Pins are tracked with `PinSet`, a `u16` bitset. Set operations like difference, union, intersection, and complement are all there, and most are `const`:

```rust
let rack = PinSet::full::<10>();
let knocked = PinSet::of([0, 5]);
let standing = rack - knocked;
assert_eq!(standing.count(), 8);
```

## Scoring

`game.scoreboard(i)` computes per-frame scoring. Frames waiting on future bonus rolls show as `Pending` rather than giving you a wrong number. Once enough rolls exist they resolve with base, bonus, and cumulative totals:

```text
Frame |   1 |   2 |   3 |   4 |   5 |   6 |   7 |   8 |   9 |  10 |
Base  |  10 |  10 |  10 |  10 |  10 |  10 |  10 |  10 |  10 |  30 |
Bonus |  20 |  20 |  20 |  20 |  20 |  20 |  20 |  20 |  10 |   0 |
Cum.  |  30 |  60 |  90 | 120 | 150 | 180 | 210 | 240 | 270 | 300 |
Total: 300
```

## Split detection

For ten-pin, `TEN_PIN_GEOMETRY` models the physical pin layout as an adjacency graph. A split is when the head pin is down and the remaining pins form disconnected groups:

```rust
let standing = PinSet::of([6, 9]); // 7-10 split
assert!(TEN_PIN_GEOMETRY.is_split(standing));
```

Detection uses BFS over the `PinSet` bitset, so there's no allocation involved.

## Examples

```sh
cargo run --bin perfect_game   # 12 strikes, 300 points
cargo run --bin multiplayer    # two-player game with scoreboards
cargo run --bin splits         # split detection on various pin leaves
cargo run --bin rulesets        # same rolls under ten-pin, candlepin, duckpin
```

## Tests

```sh
cargo test
```

This runs unit tests, property tests ([proptest](https://github.com/proptest-rs/proptest)), and compile-fail tests ([trybuild](https://github.com/dtolnay/trybuild)). The compile-fail tests check that rolling on a completed game and building with no players are type errors.

## License

Licensed under the [GNU General Public License v3.0 or later](LICENSE).

For commercial licensing options (use without GPL obligations), contact the project maintainer.
