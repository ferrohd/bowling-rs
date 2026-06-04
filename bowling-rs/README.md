# bowling-rs

Bowling game engine for Rust, generic over ruleset. Ships with ten-pin,
candlepin, and duckpin out of the box, but the `Ruleset` trait is open so you
can wire up your own variant (pin count, frame count, balls per frame,
deadwood policy, bonus scheme, spare semantics).

## Typestate

The game is a state machine encoded at the type level. `Game<R, AwaitingRoll>`
has `roll()`. `Game<R, Complete>` has `winners()`. Calling the wrong one is a
compile error, not a runtime panic.

Same goes for construction: `GameBuilder::new` takes a player name, so you
can't accidentally build a game with zero players.

```rust
// This won't compile -- roll_count doesn't exist on Game<_, Complete>
let completed: Game<TenPin, Complete> = /* ... */;
completed.roll_count(5); // error[E0599]
```

Frames use the same pattern internally. Each delivery consumes the frame and
returns a precise outcome enum (`RegAfterOne`, `FinalAfterTwo`, etc.) whose
variants are exactly the reachable next states. No `unreachable!()` arms.

## Pin tracking

Pins are a `u16` bitset (`PinSet`) with full set algebra: union, intersection,
difference, complement, subset checks, iteration. Most operations are `const`.

```rust
let rack = PinSet::full::<10>();           // const generic, checked at compile time
let knocked = PinSet::of::<2>([0, 5]);     // head pin + pin 6
let standing = rack - knocked;             // difference
assert!(!standing.contains(0));
```

You can roll against specific pins or let `roll_count(n)` pick them for you.
Fouls record which pins fell but score zero, and that zero carries through
into bonus calculations too:

```rust
let foul_roll = Roll::foul(PinSet::full::<10>());
assert_eq!(foul_roll.pin_count(), 10);  // pins did fall
assert_eq!(foul_roll.score(), 0);       // but no points
```

## Rulesets

| | Ten-pin | Candlepin | Duckpin |
|---|---|---|---|
| Balls/frame | 2 | 3 | 3 |
| Deadwood | Cleared | Remains | Cleared |
| 3-ball clearance | n/a | Spare (bonus) | AllDown (no bonus) |
| Split detection | Yes | No | No |

All three use 10 pins, 10 frames, and traditional bonus scoring (strike +2,
spare +1). The difference between candlepin and duckpin on 3-ball clears is
that candlepin treats it as a spare (earns a bonus ball), while duckpin calls
it `AllDown` and gives you a flat 10, no bonus.

Everything on the `Ruleset` trait is associated constants, so the whole thing
monomorphizes away. Zero runtime cost for the abstraction.

## Multiplayer

Turn rotation is automatic. After each frame completes, the engine advances
to the next player. Once every player finishes the same frame, it moves to
the next frame.

```rust
let game = GameBuilder::<TenPin>::new("Alice").unwrap()
    .add_player("Bob").unwrap()
    .build();

// game.current_player().name() => "Alice"
// after Alice's frame completes, it's Bob's turn for the same frame
```

## Scoring

`game.scoreboard(i)` computes cumulative scoring on the fly. Frames that are
still waiting on bonus rolls (e.g. you just threw a strike and the next two
deliveries haven't happened yet) show up as `FrameScore::Pending` rather than
being wrong. Once enough rolls exist, they flip to `FrameScore::Resolved` with
base, bonus, and cumulative totals.

```text
Frame |   1 |   2 |   3 |   4 |   5 |   6 |   7 |   8 |   9 |  10 |
Base  |  10 |  10 |  10 |  10 |  10 |  10 |  10 |  10 |  10 |  30 |
Bonus |  20 |  20 |  20 |  20 |  20 |  20 |  20 |  20 |  10 |   0 |
Cum.  |  30 |  60 |  90 | 120 | 150 | 180 | 210 | 240 | 270 | 300 |
Total: 300
```

## Split detection

`PinGeometry` is an adjacency graph over the pin deck. `TEN_PIN_GEOMETRY` has
the standard triangle layout with 18 edges. A split is defined as: head pin
down, 2+ pins still standing, and those pins form disconnected components
under the adjacency graph. Detection runs BFS using the bitset as the frontier.

```rust
let standing = PinSet::from_raw((1 << 6) | (1 << 9)); // 7-10 split
assert!(TEN_PIN_GEOMETRY.is_split(standing));
```

## Quick start

```rust
use bowling_rs::prelude::*;

let game = GameBuilder::<TenPin>::new("Alice").unwrap()
    .add_player("Bob").unwrap()
    .build();

let mut progress = Progress::AwaitingRoll(game);

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

## Examples

```sh
cargo run --bin perfect_game   # 12 strikes, 300 points
cargo run --bin multiplayer    # two-player game with scoreboards
cargo run --bin splits         # split detection on various pin leaves
cargo run --bin rulesets        # same rolls under ten-pin, candlepin, duckpin
```

## Tests

`cargo test` runs unit tests, property tests (proptest), and compile-fail
tests (trybuild). The compile-fail tests verify that rolling on a completed
game and building a game with no players are actual type errors.

## License

MIT
