use ansi_term::{Color, Style};
use anyhow::Result;
use rlifesrc_lib::{
    save::WorldSer, Config, NewState, PolyWorld, State, Status, Symmetry, ALIVE, DEAD,
};
use serde_json::{from_str, to_vec};
use std::{
    fs::{create_dir_all, File},
    io::{Read, Write},
    path::{Path, PathBuf},
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
    period: i32,
    /// Horizontal translation.
    #[structopt(short = "x", long)]
    dx: i32,
    /// Vertical translation.
    #[structopt(short = "y", long)]
    dy: i32,
    /// Symmetry.
    #[structopt(short, long, default_value = "C1")]
    symmetry: Symmetry,
    /// Rule string.
    #[structopt(short, long, default_value = "B3/S23")]
    rule: String,
    /// Maximum width.
    #[structopt(short = "w", long, default_value = "1024")]
    max_width: i32,
    /// Initial upper bound of the cell count.
    ///
    /// It will automatically decrease when a new result is found.
    #[structopt(short = "c", long, default_value = "0")]
    init_cell_count: u32,
    /// Initial height.
    ///
    /// It will automatically increase when no more result can be found.
    #[structopt(short = "h", long, default_value = "1")]
    init_height: i32,
    /// Print the world every this number of steps.
    #[structopt(short = "f", long, default_value = "5000000")]
    view_freq: u64,
    /// Save the temporary search status every this number of views.
    #[structopt(long, default_value = "100")]
    save_freq: u64,
    /// Temporary search status are saved here.
    #[structopt(long)]
    save_dir: Option<PathBuf>,
}

impl Opt {
    fn sss(&self) -> Result<Sss> {
        let cell_count = self.init_cell_count;
        let config = Config::new(self.max_width, self.init_height, self.period)
            .set_translate(self.dx, self.dy)
            .set_symmetry(self.symmetry)
            .set_rule_string(self.rule.clone())
            .set_new_state(NewState::ChooseDead)
            .set_max_cell_count(if cell_count > 0 {
                Some(cell_count - 1)
            } else {
                None
            })
            .set_reduce_max(true);
        let gen = 0;
        let world = config.world()?;
        let stopwatch = Stopwatch::start_new();
        Ok(Sss {
            cell_count,
            gen,
            world,
            stopwatch,
        })
    }
}

/// Spaceship Search
struct Sss {
    cell_count: u32,
    gen: i32,
    world: PolyWorld,
    stopwatch: Stopwatch,
}

impl Sss {
    fn from_save<P: AsRef<Path>>(save: P) -> Result<Self> {
        let mut buffer = String::new();
        File::open(&save)?.read_to_string(&mut buffer)?;
        let world = from_str::<WorldSer>(&buffer)?.world()?;
        let cell_count = world.config().max_cell_count.map(|i| i + 1).unwrap_or(0);
        let gen = 0;
        let stopwatch = Stopwatch::start_new();
        Ok(Sss {
            cell_count,
            gen,
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
                self.world.config().height,
                self.cell_count,
                self.stopwatch.elapsed()
            ),
            term_width - 1
        );
        println!("{}", Color::Yellow.paint(info));
        let width = (self.world.config().width).min(term_width as i32 - 1);
        let mut display = String::new();
        for y in 0..self.world.config().height {
            for x in 0..width {
                let state = self.world.get_cell_state((x, y, self.gen));
                match state {
                    Some(DEAD) => display.push('.'),
                    Some(ALIVE) => {
                        if self.world.is_gen_rule() {
                            display.push('A')
                        } else {
                            display.push('o')
                        }
                    }
                    Some(State(i)) => display.push((b'A' + i as u8 - 1) as char),
                    None => display.push('?'),
                };
            }
            display.push('\n');
        }
        print!("{}", style.paint(display));
    }

    fn write_pat<P: AsRef<Path>>(&self, dir: P) -> Result<()> {
        let filename = dir.as_ref().join(&format!(
            "{}P{}H{}V{}.rle",
            self.cell_count,
            self.world.config().period,
            self.world.config().dx,
            self.world.config().dy
        ));
        let mut file = File::create(filename)?;
        let mut unrle = String::new();
        let height = self.world.config().height;
        let mut width = 0;
        for y in 0..height {
            let mut line = String::new();
            for x in 0..self.world.config().width {
                let state = self.world.get_cell_state((x, y, self.gen));
                match state {
                    Some(DEAD) => {
                        if self.world.is_gen_rule() {
                            line.push('.')
                        } else {
                            line.push('b')
                        }
                    }
                    Some(ALIVE) => {
                        if self.world.is_gen_rule() {
                            line.push('A')
                        } else {
                            line.push('o')
                        }
                    }
                    Some(State(i)) => line.push((b'A' + i as u8 - 1) as char),
                    None => line.push('?'),
                };
            }
            line = line.trim_end_matches(|c| ".b?".contains(c)).to_owned();
            width = width.max(line.len() as isize);
            line.push('$');
            unrle.push_str(&line);
        }
        unrle = unrle.trim_end_matches('$').to_owned();
        unrle.push('!');
        writeln!(
            file,
            "x = {}, y = {}, rule = {}",
            width,
            height,
            self.world.config().rule_string
        )?;
        let mut line = String::new();
        let mut chars = unrle.chars().peekable();
        let mut count = 0;
        while let Some(c) = chars.next() {
            count += 1;
            if Some(&c) != chars.peek() {
                let mut run = if count > 1 {
                    count.to_string()
                } else {
                    String::new()
                };
                run.push(c);
                if line.len() + run.len() <= 70 {
                    line += &run;
                } else {
                    writeln!(file, "{}", line)?;
                    line = run;
                }
                count = 0;
            }
        }
        if line.len() < 70 {
            write!(file, "{}", line)?;
        } else {
            writeln!(file, "{}", line)?;
        }
        Ok(())
    }

    fn write_save<P: AsRef<Path>>(&self, save: P) -> Result<()> {
        let mut file = File::create(save)?;
        let json = to_vec(&self.world.ser())?;
        file.write_all(&json)?;
        Ok(())
    }

    fn search<P: AsRef<Path>, Q: AsRef<Path>>(
        &mut self,
        term_width: usize,
        dir: P,
        save: Q,
        view_freq: u64,
        save_freq: u64,
    ) -> Result<()> {
        loop {
            for _ in 0..save_freq {
                let status = self.world.search(Some(view_freq));
                match status {
                    Status::Found => {
                        let (min_gen, min_cell_count) = (0..self.world.config().period)
                            .map(|t| (t, self.world.cell_count_gen(t)))
                            .min_by_key(|p| p.1)
                            .unwrap();
                        self.gen = min_gen;
                        self.cell_count = min_cell_count;
                        self.display(term_width, Style::default());
                        self.write_pat(&dir)?;
                        self.world.set_max_cell_count(Some(self.cell_count - 1));
                        self.gen = 0;
                    }
                    Status::None => {
                        let mut config = self.world.config().clone();
                        config.height += 1;
                        self.world = config.world()?;
                        self.gen = 0;
                    }
                    Status::Initial | Status::Searching => {
                        self.display(term_width, Color::Green.normal());
                        self.gen = (self.gen + 1) % self.world.config().period;
                    }
                }
            }
            self.write_save(&save)?;
        }
    }
}

fn main() -> Result<()> {
    let term_width = dimensions().unwrap_or((80, 24)).0;
    let opt = Opt::from_args();
    create_dir_all(&opt.dir)?;
    let save_dir = opt.save_dir.as_ref().unwrap_or(&opt.dir);
    create_dir_all(&save_dir)?;
    let save = save_dir.join(&"save.json");
    let mut sss = Sss::from_save(&save).or_else(|_| opt.sss())?;
    sss.search(term_width, &opt.dir, &save, opt.view_freq, opt.save_freq)
}
