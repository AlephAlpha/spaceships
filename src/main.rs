use ansi_term::Colour::{Cyan, Green, Yellow};
use itertools::{repeat_n, Itertools};
use rlifesrc_lib::{Config, NewState, State, Status};
use term_size::dimensions;

const MAX_WIDTH: isize = 1024;
const PERIOD: isize = 4;
const VIEW_FREQ: u64 = 1_000_000;

fn main() -> Result<(), String> {
    let (term_width, _) = dimensions().ok_or("Unable to get term size.")?;
    for height in 1.. {
        let config = Config::new(MAX_WIDTH, height, PERIOD)
            .set_translate(1, 0)
            .set_new_state(NewState::Choose(State::Dead))
            .set_reduce_max(true);
        let mut search = config.set_world()?;
        loop {
            let status = search.search(Some(VIEW_FREQ));
            println!("{}", Cyan.paint(repeat_n("=", term_width - 1).join("")));
            let display = search
                .display_gen(0)
                .lines()
                .map(|l| &l[0..term_width - 1])
                .join("\n");
            match status {
                Status::Found => {
                    println!("{}", display);
                    println!("{}", search.display_gen(0));
                    return Ok(());
                }
                Status::None => {
                    println!("{}", Yellow.paint(display));
                    break;
                }
                Status::Searching => println!("{}", Green.paint(display)),
                Status::Paused => unreachable!(),
            }
        }
    }
    Ok(())
}
