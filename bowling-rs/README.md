# bowling-rs

Bowling game engine for Rust. Generic over ruleset — ships with ten-pin,
candlepin, and duckpin. Tracks scoring, multiplayer turn rotation, fouls, and
split detection.

The game state machine uses the typestate pattern, so things like rolling after
a completed game or building a game with zero players are compile errors, not
runtime ones.

## Quick look

```rust
use bowling_rs::prelude::*;

let game = GameBuilder::<TenPin>::new("Alice").unwrap()
    .add_player("Bob").unwrap()
    .build();

let mut progress = Progress::AwaitingRoll(game);

loop {
    match progress {
        Progress::AwaitingRoll(g) => {
            progress = g.roll_count(10).unwrap(); // knock down 10 pins
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

`roll_count(n)` is a convenience that picks `n` pins for you. For full control
over which specific pins get knocked and whether a foul occurred, use
`game.roll(Roll::clean(pinset))` or `game.roll(Roll::foul(pinset))` instead.

## Rulesets

| | Ten-Pin | Candlepin | Duckpin |
|---|---|---|---|
| Balls/frame | 2 | 3 | 3 |
| Deadwood | Cleared | Remains | Cleared |
| 3-ball clearance | n/a | Spare | Flat 10, no bonus |
| Split detection | Yes | No | No |

All three use 10 pins, 10 frames, and traditional bonus scoring (strike +2,
spare +1). The `Ruleset` trait is public if you want to define your own variant.

## Scoreboards

`game.scoreboard(i)` returns a `Scoreboard` that tracks per-frame scoring
mid-game. Frames waiting on bonus rolls show up as `Pending` rather than
being computed wrong. The `Display` impl gives you a text table:

```text
Frame |   1 |   2 |   3 |   4 |   5 |   6 |   7 |   8 |   9 |  10 |
Base  |  10 |  10 |  10 |  10 |  10 |  10 |  10 |  10 |  10 |  30 |
Bonus |  20 |  20 |  20 |  20 |  20 |  20 |  20 |  20 |  10 |   0 |
Cum.  |  30 |  60 |  90 | 120 | 150 | 180 | 210 | 240 | 270 | 300 |
Total: 300
```

## Split detection

`PinGeometry` models the physical adjacency of pins on the deck. The standard
10-pin triangle layout ships as `TEN_PIN_GEOMETRY`. A split is when the head pin
is down and the remaining standing pins form disconnected groups:

```rust
use bowling_rs::prelude::*;

let standing = PinSet::from_raw((1 << 6) | (1 << 9)); // 7-10 split
assert!(TEN_PIN_GEOMETRY.is_split(standing));
```

## Examples

The `bowling-examples` crate has a few runnable demos:

```sh
cargo run --bin perfect_game   # 12 strikes, 300 points
cargo run --bin multiplayer    # two-player game with scoreboards
cargo run --bin splits         # split detection on various pin leaves
cargo run --bin rulesets        # same rolls under ten-pin, candlepin, duckpin
```

## Testing

Unit tests, property-based tests (`proptest`), and compile-fail tests
(`trybuild`). `cargo test` runs everything.

## License

MIT
