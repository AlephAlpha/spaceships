// Modify this constants.
const DIR: &str = "spaceships/p3/h0v1";
const MAX_WIDTH: isize = 1024;
const PERIOD: isize = 3;
const TRANSLATION: (isize, isize) = (0, 1);
const VIEW_FREQ: u64 = 1 << 22;

use ansi_term::Color::{self, Green, White, Yellow};
use itertools::Itertools;
use rlifesrc_lib::{Config, NewState, Search, State, Status};
use std::{
    fs::{create_dir_all, OpenOptions},
    io::{Error, Write},
    path::Path,
};
use stopwatch::Stopwatch;
use term_size::dimensions;

/// Spaceship Search
struct SSS {
    cell_count: usize,
    config: Config,
    gen: isize,
    world: Box<dyn Search>,
    stopwatch: Stopwatch,
}

impl SSS {
    fn new(max_width: isize, period: isize, translation: (isize, isize)) -> Result<Self, String> {
        let cell_count = 0;
        let config = Config::new(max_width, 1, period)
            .set_translate(translation.0, translation.1)
            .set_new_state(NewState::Choose(State::Dead))
            .set_non_empty_front(true)
            .set_reduce_max(true);
        let gen = 0;
        let world = config.set_world()?;
        let stopwatch = Stopwatch::start_new();
        Ok(SSS {
            cell_count,
            config,
            gen,
            world,
            stopwatch,
        })
    }

    fn display(&self, term_width: usize, color: Color) {
        let info = format!(
            "{:=<1$}",
            format!(
                "=GEN:{}==HEIGHT:{}==CELLS:{}==TIME:{:.2?}",
                self.gen,
                self.config.height,
                self.cell_count,
                self.stopwatch.elapsed()
            ),
            term_width - 1
        );
        println!("{}", Yellow.paint(info));
        let display = self
            .world
            .display_gen(0)
            .lines()
            .map(|l| &l[0..term_width - 1])
            .join("\n");
        println!("{}", color.paint(display));
    }

    fn write_pat<P: AsRef<Path>>(&self, dir: &P) -> Result<(), Error> {
        create_dir_all(dir)?;
        let filename = dir.as_ref().join(&format!(
            "{}P{}H{}V{}.rle",
            self.cell_count, self.config.period, self.config.dx, self.config.dy
        ));
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(filename)?;
        let display = self.world.display_gen(self.gen);
        let mut lines = display.lines().map(|l| l.trim_end_matches('.'));
        let width = lines.clone().map(|l| l.len()).max().unwrap_or(0);
        let height = lines.clone().count();
        writeln!(
            file,
            "x = {}, y = {}, rule = {}",
            width, height, self.config.rule_string
        )?;
        let (mut char, mut n) = (None, 0);
        for c in lines.join("\n").trim_end().chars() {
            if char == Some(c) {
                n += 1;
            } else if n > 0 && char.is_some() {
                write!(file, "{}", n)?;
                match char {
                    Some('.') => write!(file, "b")?,
                    Some('O') => write!(file, "o")?,
                    Some('\n') => write!(file, "$")?,
                    _ => unreachable!(),
                }
                char = Some(c);
                n = 1;
            } else {
                char = Some(c);
                n = 1;
            }
        }
        if n > 0 && char.is_some() {
            write!(file, "{}", n)?;
            match char {
                Some('.') => write!(file, "b")?,
                Some('O') => write!(file, "o")?,
                Some('\n') => write!(file, "$")?,
                _ => unreachable!(),
            }
        }
        write!(file, "!")
    }

    fn search<P: AsRef<Path>>(&mut self, term_width: usize, dir: &P) -> Result<(), String> {
        loop {
            let status = self.world.search(Some(VIEW_FREQ));
            self.gen = (self.gen + 1) % PERIOD;
            match status {
                Status::Found => {
                    let (min_gen, min_cell_count) = (0..PERIOD)
                        .map(|t| (t, self.world.cell_count(t)))
                        .min_by_key(|p| p.1)
                        .unwrap();
                    self.gen = min_gen;
                    self.cell_count = min_cell_count;
                    self.display(term_width, White);
                    self.write_pat(dir).map_err(|e| e.to_string())?;
                    self.config.max_cell_count = Some(self.cell_count - 1);
                }
                Status::None => {
                    self.config.height += 1;
                    self.world = self.config.set_world()?;
                    self.gen = 0;
                }
                Status::Searching => self.display(term_width, Green),
                Status::Paused => unreachable!(),
            }
        }
    }
}

fn main() -> Result<(), String> {
    let term_width = dimensions().unwrap_or((80, 24)).0;
    let mut sss = SSS::new(MAX_WIDTH, PERIOD, TRANSLATION)?;
    sss.search(term_width, &DIR)
}
