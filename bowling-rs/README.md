# bowling-rs

Bowling game engine for Rust, generic over ruleset. Ships with ten-pin,
candlepin, and duckpin.

The game is a typestate machine, so rolling after a completed game or building
a game with zero players won't compile.

## Usage

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

`roll_count(n)` picks `n` pins for you. If you need to specify which pins
and whether it was a foul, use `game.roll(Roll::clean(pinset))` or
`game.roll(Roll::foul(pinset))`.

## Rulesets

| | Ten-pin | Candlepin | Duckpin |
|---|---|---|---|
| Balls/frame | 2 | 3 | 3 |
| Deadwood | Cleared | Remains | Cleared |
| 3-ball clearance | n/a | Spare | Flat 10, no bonus |
| Split detection | Yes | No | No |

All three use 10 pins, 10 frames, and standard bonus scoring (strike +2,
spare +1). Implement the `Ruleset` trait to add your own.

## Scoreboards

`game.scoreboard(i)` returns per-frame scoring mid-game. Frames still waiting
on bonus rolls show as `Pending`. The `Display` impl gives you a text table:

```text
Frame |   1 |   2 |   3 |   4 |   5 |   6 |   7 |   8 |   9 |  10 |
Base  |  10 |  10 |  10 |  10 |  10 |  10 |  10 |  10 |  10 |  30 |
Bonus |  20 |  20 |  20 |  20 |  20 |  20 |  20 |  20 |  10 |   0 |
Cum.  |  30 |  60 |  90 | 120 | 150 | 180 | 210 | 240 | 270 | 300 |
Total: 300
```

## Split detection

`PinGeometry` models pin adjacency on the deck. The standard 10-pin triangle
is `TEN_PIN_GEOMETRY`. A split is when the head pin is down and the remaining
pins form disconnected groups:

```rust
use bowling_rs::prelude::*;

let standing = PinSet::from_raw((1 << 6) | (1 << 9)); // 7-10 split
assert!(TEN_PIN_GEOMETRY.is_split(standing));
```

## Examples

```sh
cargo run --bin perfect_game   # 12 strikes, 300 points
cargo run --bin multiplayer    # two-player game with scoreboards
cargo run --bin splits         # split detection on various pin leaves
cargo run --bin rulesets        # same rolls under ten-pin, candlepin, duckpin
```

## Tests

`cargo test` runs unit tests, property tests (proptest), and compile-fail
tests (trybuild).

## License

MIT
