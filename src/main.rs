/// The world is printed every `VIEW_FREQ` steps.
const VIEW_FREQ: u64 = 1 << 22;

use ansi_term::{Color, Style};
use itertools::Itertools;
use rlifesrc_lib::{Config, NewState, Search, State, Status, Symmetry};
use std::{
    fs::{create_dir_all, File},
    io::{Error, Write},
    path::Path,
};
use stopwatch::Stopwatch;
use structopt::StructOpt;
use term_size::dimensions;

#[derive(Clone, Debug, StructOpt)]
#[structopt(name = "basic")]
struct Opt {
    /// Search results will be saved in this directory.
    #[structopt(short, long)]
    dir: String,
    #[structopt(short, long, default_value = "1024")]
    max_width: isize,
    #[structopt(short, long)]
    period: isize,
    #[structopt(short, long, default_value = "0")]
    dx: isize,
    #[structopt(short, long, default_value = "42")]
    dy: isize,
    #[structopt(short, long, default_value = "C1")]
    symmetry: Symmetry,
    #[structopt(short, long, default_value = "B3/S23")]
    rule: String,
}

/// Spaceship Search
struct SSS {
    cell_count: usize,
    config: Config,
    gen: isize,
    period: isize,
    world: Box<dyn Search>,
    stopwatch: Stopwatch,
}

impl SSS {
    fn new(opt: Opt) -> Result<Self, String> {
        let cell_count = 0;
        let config = Config::new(opt.max_width, 1, opt.period)
            .set_translate(opt.dx, opt.dy)
            .set_symmetry(opt.symmetry)
            .set_rule_string(opt.rule)
            .set_new_state(NewState::Choose(State::Dead))
            .set_non_empty_front(true)
            .set_reduce_max(true);
        let gen = 0;
        let period = opt.period;
        let world = config.set_world()?;
        let stopwatch = Stopwatch::start_new();
        Ok(SSS {
            cell_count,
            config,
            gen,
            period,
            world,
            stopwatch,
        })
    }

    fn display(&self, term_width: usize, style: Style) {
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
        println!("{}", Color::Yellow.paint(info));
        let display = self
            .world
            .display_gen(self.gen)
            .lines()
            .map(|l| &l[0..term_width - 1])
            .join("\n");
        println!("{}", style.paint(display));
    }

    fn write_pat<P: AsRef<Path>>(&self, dir: &P) -> Result<(), Error> {
        create_dir_all(dir)?;
        let filename = dir.as_ref().join(&format!(
            "{}P{}H{}V{}.rle",
            self.cell_count, self.config.period, self.config.dx, self.config.dy
        ));
        let mut file = File::create(filename)?;
        let pat = self.world.display_gen(self.gen);
        let mut lines = pat.lines().map(|l| l.trim_end_matches('.'));
        let width = lines.clone().map(|l| l.len()).max().unwrap_or(0);
        let height = lines.clone().count();
        writeln!(
            file,
            "x = {}, y = {}, rule = {}",
            width, height, self.config.rule_string
        )?;
        let (mut char, mut n) = (None, 0);
        let mut line = String::new();
        for c in lines.join("\n").trim_end().chars() {
            if char == Some(c) {
                n += 1;
            } else if n > 0 && char.is_some() {
                let mut tally = if n > 1 {
                    format!("{}", n)
                } else {
                    String::new()
                };
                match char {
                    Some('.') => tally.push('b'),
                    Some('O') => tally.push('o'),
                    Some('\n') => tally.push('$'),
                    _ => unreachable!(),
                }
                if line.len() + tally.len() <= 70 {
                    line += &tally;
                } else {
                    writeln!(file, "{}", line)?;
                    line = tally;
                }
                char = Some(c);
                n = 1;
            } else {
                char = Some(c);
                n = 1;
            }
        }
        let mut tally = if n > 1 {
            format!("{}", n)
        } else {
            String::new()
        };
        match char {
            Some('.') => tally.push('b'),
            Some('O') => tally.push('o'),
            Some('\n') => tally.push('$'),
            _ => unreachable!(),
        }
        if line.len() + tally.len() <= 70 {
            line += &tally;
        } else {
            writeln!(file, "{}", line)?;
            line = tally;
        }
        if line.len() < 70 {
            write!(file, "{}", line)?;
        } else {
            writeln!(file, "{}", line)?;
        }
        writeln!(file, "!")
    }

    fn search<P: AsRef<Path>>(&mut self, term_width: usize, dir: &P) -> Result<(), String> {
        loop {
            let status = self.world.search(Some(VIEW_FREQ));
            match status {
                Status::Found => {
                    let (min_gen, min_cell_count) = (0..self.period)
                        .map(|t| (t, self.world.cell_count(t)))
                        .min_by_key(|p| p.1)
                        .unwrap();
                    self.gen = min_gen;
                    self.cell_count = min_cell_count;
                    self.display(term_width, Style::default());
                    self.write_pat(dir).map_err(|e| e.to_string())?;
                    self.config.max_cell_count = Some(self.cell_count - 1);
                    self.gen = 0;
                }
                Status::None => {
                    self.config.height += 1;
                    self.world = self.config.set_world()?;
                    self.gen = 0;
                }
                Status::Searching => {
                    self.display(term_width, Color::Green.normal());
                    self.gen = (self.gen + 1) % self.period;
                }
                Status::Paused => unreachable!(),
            }
        }
    }
}

fn main() -> Result<(), String> {
    let term_width = dimensions().unwrap_or((80, 24)).0;
    let opt = Opt::from_args();
    let dir = opt.dir.clone();
    let mut sss = SSS::new(opt)?;
    sss.search(term_width, &dir)
}
