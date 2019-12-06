use ansi_term::{Color, Style};
use itertools::Itertools;
use rlifesrc_lib::{Config, NewState, Search, State, Status, Symmetry};
use std::{
    fs::{create_dir_all, File},
    io::{Error, Write},
    path::PathBuf,
};
use stopwatch::Stopwatch;
use structopt::StructOpt;
use term_size::dimensions;

#[derive(Clone, Debug, StructOpt)]
#[structopt(
    no_version,
    author = "AlephAlpha",
    about = "Search for spaceships in Conway's Game of Life using the rlifesrc lib.\n\
             \n\
             It starts from a given minimum height, and an optional upper bound of \
             the cell count.\n\
             \n\
             When a new result is found, it will reduce the upper bound to the cell \
             count of this result minus 1 (even if there is no initial upper bound).\n\
             \n\
             When no more result can be found, it will increase the height by 1 and \
             continue the search.\n\
             \n\
             Spaceships with period `p`, speed `(x,y)c/p`, and `n` cells are saved \
             in the file `{n}P{p}H{x}V{y}.rle`.\n\
             \n\
             Press `Ctrl-C` to abort."
)]
struct Opt {
    /// Search results are saved here.
    #[structopt(short, long)]
    dir: PathBuf,
    /// Period.
    #[structopt(short, long)]
    period: isize,
    /// Horizontal translation.
    #[structopt(short = "x", long)]
    dx: isize,
    /// Vertical translation.
    #[structopt(short = "y", long)]
    dy: isize,
    /// Symmetry.
    #[structopt(short, long, default_value = "C1")]
    symmetry: Symmetry,
    /// Rule string.
    #[structopt(short, long, default_value = "B3/S23")]
    rule: String,
    /// Maximum width.
    #[structopt(short = "w", long, default_value = "1024")]
    max_width: isize,
    /// Initial upper bound of the cell count.
    ///
    /// It will automatically decrease when a new result is found.
    #[structopt(short = "c", long, default_value = "0")]
    init_cell_count: usize,
    /// Initial height.
    ///
    /// It will automatically increase when no more result can be found.
    #[structopt(short = "h", long, default_value = "1")]
    init_height: isize,
    /// Print the world every this number of steps.
    #[structopt(short = "f", long, default_value = "5000000")]
    view_freq: u64,
}

impl Opt {
    fn sss(&self) -> Result<SSS, String> {
        let cell_count = self.init_cell_count;
        let config = Config::new(self.max_width, self.init_height, self.period)
            .set_translate(self.dx, self.dy)
            .set_symmetry(self.symmetry)
            .set_rule_string(self.rule.clone())
            .set_new_state(NewState::Choose(State::Dead))
            .set_non_empty_front(true)
            .set_max_cell_count(if cell_count > 0 {
                Some(cell_count - 1)
            } else {
                None
            })
            .set_reduce_max(true);
        let gen = 0;
        let period = self.period;
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

    fn write_pat(&self, dir: &PathBuf) -> Result<(), Error> {
        create_dir_all(dir)?;
        let filename = dir.join(&format!(
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

    fn search(&mut self, term_width: usize, dir: &PathBuf, view_freq: u64) -> Result<(), String> {
        loop {
            let status = self.world.search(Some(view_freq));
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
    let mut sss = opt.sss()?;
    sss.search(term_width, &opt.dir, opt.view_freq)
}
