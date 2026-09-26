const CELL_ALIVE_THRESHOLD: f32 = 0.3;

#[derive(Clone, Copy, Debug, Default)]
struct Cell {
    alive: bool,
    heat: u8,
}

impl Cell {
    fn new(alive: bool) -> Self {
        let heat = if alive { 255 } else { 0 };
        Self { alive, heat }
    }

    fn cool_if_dead(&mut self, subtract_count: u8) {
        if !self.alive && self.heat > 0 {
            self.heat = self.heat.saturating_sub(subtract_count);
        }
    }

    fn set(&mut self, alive: bool) {
        self.alive = alive;

        if self.alive {
            self.heat = 255;
        }
    }
}

pub(crate) struct Grid {
    grid: Vec<Cell>,
    width: usize,
    height: usize,
    cycle_checkpoint: Vec<bool>,
    cycle_power: usize,
    cycle_length: usize,
}

impl Grid {
    pub(crate) fn new_empty(width: usize, height: usize) -> Self {
        let size = width.checked_mul(height).expect("Grid too big (overflow)");
        Self {
            grid: vec![Cell::default(); size],
            width,
            height,
            cycle_checkpoint: vec![false; size],
            cycle_power: 1,
            cycle_length: 0,
        }
    }

    pub(crate) fn update(&mut self) -> bool {
        let mut grid_tmp = self.grid.clone();
        let mut changed = false;

        for x in 0..self.width {
            for y in 0..self.height {
                let neighbors_alive = self.count_neighbors(x, y);

                if let Some(cell) = self.grid_idx(x, y) {
                    let was_alive = self.grid[cell].alive;
                    let is_alive = neighbors_alive == 3 || (was_alive && neighbors_alive == 2);

                    grid_tmp[cell].set(is_alive);
                    if !is_alive {
                        grid_tmp[cell].cool_if_dead(50);
                    }
                    changed |= was_alive != is_alive;
                } else {
                    unreachable!("grid coordinates are within bounds");
                }
            }
        }

        std::mem::swap(&mut grid_tmp, &mut self.grid);
        changed && !self.repeats_previous_state()
    }

    pub(crate) fn randomize(&mut self) {
        let mut rand: randomize::PCG32 = generate_seed().into();

        for cell in &mut self.grid {
            let alive = randomize::f32_half_open_right(rand.next_u32()) > CELL_ALIVE_THRESHOLD;
            *cell = Cell::new(alive);
        }

        self.normalize(5);
        self.reset_cycle_detection();
    }

    pub(crate) fn randomly_kill(&mut self) -> u32 {
        let mut rand: randomize::PCG32 = generate_seed().into();
        let mut kill_count = 0;

        for cell in &mut self.grid {
            if cell.alive {
                let kill = randomize::f32_half_open_right(rand.next_u32()) > CELL_ALIVE_THRESHOLD;
                if kill {
                    cell.set(false);
                    kill_count += 1;
                }
            }
        }

        self.reset_cycle_detection();
        kill_count
    }

    pub(crate) fn cells(&self) -> impl Iterator<Item = (bool, u8)> + '_ {
        self.grid.iter().map(|cell| (cell.alive, cell.heat))
    }

    pub(crate) fn alive_count(&self) -> usize {
        self.grid.iter().filter(|cell| cell.alive).count()
    }

    pub(crate) fn cell_count(&self) -> usize {
        self.grid.len()
    }

    pub(crate) fn toggle(&mut self, x: isize, y: isize) -> bool {
        if let Some(index) = self.grid_idx(x, y) {
            let alive = !self.grid[index].alive;
            self.grid[index].set(alive);
            self.reset_cycle_detection();
            alive
        } else {
            false
        }
    }

    pub(crate) fn set_line(&mut self, x0: isize, y0: isize, x1: isize, y1: isize, alive: bool) {
        let x0 = x0.max(0).min(self.width as isize);
        let y0 = y0.max(0).min(self.height as isize);
        let mut changed = false;
        for (x, y) in line_drawing::Bresenham::new((x0, y0), (x1, y1)) {
            if let Some(index) = self.grid_idx(x, y) {
                if self.grid[index].alive != alive {
                    self.grid[index].set(alive);
                    changed = true;
                }
            } else {
                break;
            }
        }
        if changed {
            self.reset_cycle_detection();
        }
    }

    fn count_neighbors(&self, x: usize, y: usize) -> usize {
        let (xm1, xp1) = if x == 0 {
            (self.width - 1, x + 1)
        } else if x == self.width - 1 {
            (x - 1, 0)
        } else {
            (x - 1, x + 1)
        };

        let (ym1, yp1) = if y == 0 {
            (self.height - 1, y + 1)
        } else if y == self.height - 1 {
            (y - 1, 0)
        } else {
            (y - 1, y + 1)
        };

        self.grid[xm1 + ym1 * self.width].alive as usize
            + self.grid[x + ym1 * self.width].alive as usize
            + self.grid[xp1 + ym1 * self.width].alive as usize
            + self.grid[xm1 + y * self.width].alive as usize
            + self.grid[xp1 + y * self.width].alive as usize
            + self.grid[xm1 + yp1 * self.width].alive as usize
            + self.grid[x + yp1 * self.width].alive as usize
            + self.grid[xp1 + yp1 * self.width].alive as usize
    }

    fn normalize(&mut self, generations: usize) {
        self.randomly_kill();

        for _ in 0..generations {
            let _ = self.update();
        }

        for cell in &mut self.grid {
            if !cell.alive {
                cell.heat = 0;
            }
        }
    }

    fn repeats_previous_state(&mut self) -> bool {
        self.cycle_length += 1;
        let repeats = self
            .grid
            .iter()
            .map(|cell| cell.alive)
            .eq(self.cycle_checkpoint.iter().copied());

        if !repeats && self.cycle_length == self.cycle_power {
            self.cycle_checkpoint.clear();
            self.cycle_checkpoint
                .extend(self.grid.iter().map(|cell| cell.alive));
            self.cycle_power = self.cycle_power.saturating_mul(2);
            self.cycle_length = 0;
        }

        repeats
    }

    fn reset_cycle_detection(&mut self) {
        self.cycle_checkpoint.clear();
        self.cycle_checkpoint
            .extend(self.grid.iter().map(|cell| cell.alive));
        self.cycle_power = 1;
        self.cycle_length = 0;
    }

    fn grid_idx<I: TryInto<usize>>(&self, x: I, y: I) -> Option<usize> {
        if let (Ok(x), Ok(y)) = (x.try_into(), y.try_into()) {
            if x < self.width && y < self.height {
                Some(x + y * self.width)
            } else {
                None
            }
        } else {
            None
        }
    }
}

fn generate_seed() -> (u64, u64) {
    use byteorder::{ByteOrder, NativeEndian};
    use getrandom::getrandom;

    let mut seed = [0_u8; 16];
    getrandom(&mut seed).expect("failed to getrandom");

    (
        NativeEndian::read_u64(&seed[0..8]),
        NativeEndian::read_u64(&seed[8..16]),
    )
}

#[cfg(test)]
#[path = "../Tests/Unit/life.rs"]
mod tests;
