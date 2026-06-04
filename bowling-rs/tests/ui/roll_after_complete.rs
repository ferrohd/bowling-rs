// This test proves that calling .roll() on a completed game does not compile.
// The typestate pattern makes this a compile-time error.

use bowling_rs::prelude::*;

fn main() {
    let game: Game<TenPin, Complete> = todo!("assume we have a completed game");
    // This line should fail to compile: `roll` is not defined for
    // Game<TenPin, Complete>.
    let _ = game.roll_count(5);
}
