//! The perfect game: 12 strikes, 300 points.

use bowling_rs::prelude::*;

fn main() {
    println!("=== Perfect Game (Ten-Pin) ===\n");

    let game = GameBuilder::<TenPin>::new("Alice").unwrap().build();
    let mut progress = Progress::AwaitingRoll(game);

    let mut roll_num = 0;
    loop {
        match progress {
            Progress::AwaitingRoll(g) => {
                roll_num += 1;
                let frame = g.current_frame_number();
                let ball = g.current_ball_number();
                println!("  Roll {roll_num:>2}: frame {frame:>2}, ball {ball} — STRIKE!");
                progress = g.roll_count(10).unwrap();
            }
            Progress::Complete(g) => {
                println!("\n{}\n", g.scoreboard(0));
                assert_eq!(g.scoreboard(0).total, 300);
                println!("Final score: 300. Perfect.");
                break;
            }
        }
    }
}
