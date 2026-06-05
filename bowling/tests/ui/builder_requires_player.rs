// This test proves that GameBuilder::new() requires a Player argument.
// Calling it with no arguments is a compile error, so you cannot construct a
// game with zero players.

use bowling::prelude::*;

fn main() {
    let _builder = GameBuilder::<TenPin>::new();
}
