//! Self-playing Snake and Tetris for the LED panel.
//!
//! Both games run themselves. The panel has no input device and the app exposes no
//! game controls, so these are animated demos rather than playable games - which
//! is also what makes them safe to leave running unattended.
//!
//! The panel is tiny: 3 lanes wide by ~13 rows. That shapes both designs. Snake is
//! a narrow vertical corridor. Tetris only fits because every tetromino except the
//! I-piece is 3 columns wide, so the I-piece is kept vertical.
//!
//! Both games advance from real elapsed seconds rather than per-frame steps, so the
//! play speed is independent of the engine tick rate and of the `game_speed`
//! multiplier. Each game own its own `step_secs`, derived from that multiplier.
//!
//! A game's frame is exposed through the [`Board`] trait, which lets the engine
//! publish a live preview to the UI through the same palette the LEDs use.

use crate::openrgb::RgbColor;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Palette indices returned by `cell()`. The engine maps these to colours so the
/// games stay independent of brightness handling.
pub const PAL_BODY: u8 = 0;
pub const PAL_HEAD: u8 = 1;
pub const PAL_FOOD: u8 = 2;
/// Tetris piece colours occupy `PAL_PIECE_BASE .. PAL_PIECE_BASE + 7`.
pub const PAL_PIECE_BASE: u8 = 3;
/// The two colours a completed line flashes between while it clears.
pub const PAL_CLEAR_A: u8 = 10;
pub const PAL_CLEAR_B: u8 = 11;
/// Highest palette index this module can ever return. Used by the tests to catch
/// an index the engine would render as black.
#[cfg(test)]
pub const PAL_MAX: u8 = PAL_CLEAR_B;

/// Small xorshift PRNG. A `rand` dependency is not worth it for cosmetics.
#[derive(Clone, Copy)]
struct Rng(u32);

impl Rng {
    fn new(seed: u32) -> Self {
        Self(seed | 1)
    }

    fn next(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }

    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() as usize) % n
        }
    }
}

fn mix_white(c: RgbColor, amount: f32) -> RgbColor {
    let a = amount.clamp(0.0, 1.0);
    let f = |v: u8| (v as f32 + (255.0 - v as f32) * a) as u8;
    RgbColor::new(f(c.r), f(c.g), f(c.b))
}

/// Map a palette index to a colour, using the user's accent for snake parts.
pub fn game_color(idx: u8, accent: RgbColor) -> RgbColor {
    match idx {
        PAL_HEAD => mix_white(accent, 0.75),
        PAL_FOOD => RgbColor::new(255, 60, 60),
        // Piece colours, indexed from PAL_PIECE_BASE. Named so the arms are
        // actual patterns rather than expressions.
        i if i == PAL_PIECE_BASE => RgbColor::new(0, 220, 255), // I
        i if i == PAL_PIECE_BASE + 1 => RgbColor::new(255, 220, 0), // O
        i if i == PAL_PIECE_BASE + 2 => RgbColor::new(170, 0, 255), // T
        i if i == PAL_PIECE_BASE + 3 => RgbColor::new(0, 220, 80), // S
        i if i == PAL_PIECE_BASE + 4 => RgbColor::new(255, 40, 60), // Z
        i if i == PAL_PIECE_BASE + 5 => RgbColor::new(60, 90, 255), // J
        i if i == PAL_PIECE_BASE + 6 => RgbColor::new(255, 140, 0), // L
        PAL_CLEAR_A => RgbColor::new(255, 255, 255),             // clear flash, bright
        PAL_CLEAR_B => RgbColor::new(255, 60, 60),               // clear flash, hot
        _ => accent,                                            // snake body
    }
}

// ---------------------------------------------------------------- board interface

/// Which game a snapshot came from.
///
/// The UI uses this to discard a frame from the other game. Without it, switching
/// between Snake and Tetris leaves the previous game's board on screen until the
/// first new frame arrives - and if the stream stalls, indefinitely.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameKind {
    Snake,
    Tetris,
}

/// One frame of a game, in palette indices, for the UI preview.
///
/// Cell order is `LANE-MAJOR`, matching the panel preview in the frontend: the
/// outer slice is the lane, the inner slice is the row running up the fan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSnapshot {
    /// Which game produced this frame.
    pub game: GameKind,
    /// `lanes` entries, each `rows` long. `None` is an unlit cell.
    pub lanes: Vec<Vec<Option<u8>>>,
    /// Completed lines so far. Always 0 for snake, which has no lines.
    pub cleared: u32,
    /// True while a completed line is flashing.
    pub flashing: bool,
    /// Rows currently flashing. Empty unless `flashing` is true.
    pub clearing_rows: Vec<usize>,
}

/// Uniform view over the two games, so the engine can render and publish them
/// without matching on the mode at every call site.
///
/// Advancing is deliberately NOT part of this trait: each game's timing comes from
/// the `Games` speed multiplier, which would otherwise need to be threaded through
/// every `step` call.
pub trait Board {
    /// Palette index for a panel cell: `row` is vertical, `col` is the lane.
    fn cell(&self, row: usize, col: usize) -> Option<u8>;

    /// Board shape as (lanes, rows).
    fn dims(&self) -> (usize, usize);

    /// Snapshot the current frame as lane-major palette indices.
    fn snapshot(&self) -> GameSnapshot {
        let (lanes, rows) = self.dims();
        GameSnapshot {
            game: GameKind::Snake,
            lanes: (0..lanes)
                .map(|c| (0..rows).map(|r| self.cell(r, c)).collect())
                .collect(),
            cleared: 0,
            flashing: false,
            clearing_rows: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------- snake

pub struct SnakeGame {
    cols: usize,
    rows: usize,
    /// Front is the tail, back is the head.
    body: VecDeque<(usize, usize)>,
    dir: (isize, isize),
    food: (usize, usize),
    rng: Rng,
    accum: f32,
    /// Seconds per move, already divided by the user's speed multiplier.
    step_secs: f32,
}

/// Seconds between snake moves at 1x speed: a full 13-row traversal takes ~2.9 s.
pub const SNAKE_STEP_SECS: f32 = 0.22;

impl SnakeGame {
    pub fn new(cols: usize, rows: usize) -> Self {
        let mut g = Self {
            cols,
            rows,
            body: VecDeque::new(),
            dir: (0, 1),
            food: (0, 0),
            rng: Rng::new(0x9E37_79B9),
            accum: 0.0,
            step_secs: SNAKE_STEP_SECS,
        };
        g.reset();
        g
    }

    pub fn dims(&self) -> (usize, usize) {
        (self.cols, self.rows)
    }

    fn reset(&mut self) {
        self.body.clear();
        let mid = self.cols / 2;
        for y in 0..3.min(self.rows) {
            self.body.push_back((mid, y));
        }
        self.dir = (0, 1); // heading up
        self.accum = 0.0;
        self.place_food();
    }

    fn occupied(&self, x: usize, y: usize) -> bool {
        self.body.contains(&(x, y))
    }

    fn place_food(&mut self) {
        let free: Vec<(usize, usize)> = (0..self.rows)
            .flat_map(|y| (0..self.cols).map(move |x| (x, y)))
            .filter(|&(x, y)| !self.occupied(x, y))
            .collect();

        if free.is_empty() {
            // Board is full: start over so the demo keeps moving.
            self.reset();
            return;
        }
        self.food = free[self.rng.below(free.len())];
    }

    pub fn step(&mut self, dt: f32) {
        if self.cols == 0 || self.rows == 0 || self.step_secs <= 0.0 {
            return;
        }
        self.accum += dt;
        // Cap the catch-up so a long stall (window minimised, debugger paused)
        // cannot make the snake teleport through a dozen moves at once.
        let max_steps = 8;
        let mut steps = 0;
        while self.accum >= self.step_secs && steps < max_steps {
            self.accum -= self.step_secs;
            self.advance();
            steps += 1;
        }
        if steps == max_steps {
            self.accum = 0.0;
        }
    }

    fn advance(&mut self) {
        let Some(dir) = self.choose_dir() else {
            self.reset();
            return;
        };
        self.dir = dir;

        let head = *self.body.back().unwrap();
        let (nx, ny) = (head.0 as isize + dir.0, head.1 as isize + dir.1);
        if nx < 0 || ny < 0 || nx >= self.cols as isize || ny >= self.rows as isize {
            self.reset();
            return;
        }

        let next = (nx as usize, ny as usize);
        let eating = next == self.food;
        // The tail vacates this tick unless we're growing, so it isn't a collision.
        if !eating {
            self.body.pop_front();
        }
        if self.body.contains(&next) {
            self.reset();
            return;
        }
        self.body.push_back(next);

        if eating {
            self.place_food();
        }
    }

    /// Pick the next direction with a greedy search plus a survival check.
    ///
    /// The snake prefers moves that close the distance to the food, but a move is
    /// rejected outright if it would leave less free space than the body needs -
    /// otherwise it happily curls into a pocket and dies immediately.
    fn choose_dir(&mut self) -> Option<(isize, isize)> {
        let head = *self.body.back()?;
        let dirs = [(0isize, 1isize), (1, 0), (0, -1), (-1, 0)];
        let reverse = (-self.dir.0, -self.dir.1);

        let mut best: Option<((isize, isize), i32, i32)> = None;

        for d in dirs {
            if d == reverse {
                continue;
            }
            let (nx, ny) = (head.0 as isize + d.0, head.1 as isize + d.1);
            if nx < 0 || ny < 0 || nx >= self.cols as isize || ny >= self.rows as isize {
                continue;
            }
            let n = (nx as usize, ny as usize);

            let mut sim = self.body.clone();
            let eating = n == self.food;
            if !eating {
                sim.pop_front();
            }
            if sim.contains(&n) {
                continue;
            }
            sim.push_back(n);

            let free = count_free(&sim, self.cols, self.rows, n);
            if free < sim.len() {
                continue; // would not leave room to survive
            }

            let dist = (n.0 as i32 - self.food.0 as i32).abs()
                + (n.1 as i32 - self.food.1 as i32).abs();
            let better = match best {
                None => true,
                Some((_, bd, bf)) => dist < bd || (dist == bd && free as i32 > bf),
            };
            if better {
                best = Some((d, dist, free as i32));
            }
        }

        if let Some((d, _, _)) = best {
            return Some(d);
        }

        // Every option looks like a trap. Take any legal move and let the reset
        // handle it, rather than freezing on the spot.
        for d in dirs {
            if d == reverse {
                continue;
            }
            let (nx, ny) = (head.0 as isize + d.0, head.1 as isize + d.1);
            if nx < 0 || ny < 0 || nx >= self.cols as isize || ny >= self.rows as isize {
                continue;
            }
            let n = (nx as usize, ny as usize);
            let mut sim = self.body.clone();
            if n != self.food {
                sim.pop_front();
            }
            if !sim.contains(&n) {
                return Some(d);
            }
        }
        None
    }

    /// Palette index for a panel cell. `row` is vertical, `col` is the lane.
    pub fn cell(&self, row: usize, col: usize) -> Option<u8> {
        if row >= self.rows || col >= self.cols {
            return None;
        }
        let p = (col, row);
        if p == self.food {
            return Some(PAL_FOOD);
        }
        if self.body.back() == Some(&p) {
            return Some(PAL_HEAD);
        }
        if self.body.contains(&p) {
            return Some(PAL_BODY);
        }
        None
    }
}

impl Board for SnakeGame {
    fn cell(&self, row: usize, col: usize) -> Option<u8> {
        SnakeGame::cell(self, row, col)
    }

    fn dims(&self) -> (usize, usize) {
        self.dims()
    }
}

/// Flood fill the open cells reachable from `from`, treating the body as walls.
fn count_free(
    body: &VecDeque<(usize, usize)>,
    cols: usize,
    rows: usize,
    from: (usize, usize),
) -> usize {
    if cols == 0 || rows == 0 {
        return 0;
    }
    let mut blocked = vec![false; cols * rows];
    for &(x, y) in body {
        // The head occupies `from`, and we want to count it as reachable space.
        if (x, y) != from {
            blocked[y * cols + x] = true;
        }
    }

    let mut stack = vec![from];
    blocked[from.1 * cols + from.0] = true;
    let mut count = 0;

    while let Some((x, y)) = stack.pop() {
        count += 1;
        for (dx, dy) in [(0isize, 1isize), (1, 0), (0, -1), (-1, 0)] {
            let (nx, ny) = (x as isize + dx, y as isize + dy);
            if nx < 0 || ny < 0 || nx >= cols as isize || ny >= rows as isize {
                continue;
            }
            let (nx, ny) = (nx as usize, ny as usize);
            if !blocked[ny * cols + nx] {
                blocked[ny * cols + nx] = true;
                stack.push((nx, ny));
            }
        }
    }
    count
}

// ---------------------------------------------------------------- tetris

/// Tetromino shapes as (x, y) offsets, y increasing upward.
///
/// The I-piece is deliberately vertical: it is the only shape 4 cells long, and
/// the panel is 3 columns wide, so a horizontal I would never fit.
const SHAPES: [&[(i8, i8)]; 7] = [
    &[(0, 0), (0, 1), (0, 2), (0, 3)], // I
    &[(0, 0), (1, 0), (0, 1), (1, 1)], // O
    &[(0, 0), (1, 0), (2, 0), (1, 1)], // T
    &[(1, 0), (2, 0), (0, 1), (1, 1)], // S
    &[(0, 0), (1, 0), (1, 1), (2, 1)], // Z
    &[(0, 0), (0, 1), (1, 1), (2, 1)], // J
    &[(2, 0), (0, 1), (1, 1), (2, 1)], // L
];

/// Seconds between gravity steps at 1x speed. A 13-row drop takes ~3.6 s.
pub const TETRIS_STEP_SECS: f32 = 0.28;
/// How long a fresh piece sits at the top before it starts to move. Long enough
/// to register what the shape is, short enough not to feel stalled.
const TETRIS_SPAWN_HOLD_SECS: f32 = 0.16;
/// Seconds between the quarter turns of the rotation animation.
const TETRIS_ROTATE_STEP_SECS: f32 = 0.09;
/// How long a completed line flashes before it is removed.
const TETRIS_CLEAR_FLASH_SECS: f32 = 0.45;
/// Flash blinks per second while a completed line is clearing.
const TETRIS_CLEAR_BLINKS_PER_SEC: f32 = 8.0;

fn shape_width(cells: &[(i8, i8)]) -> i8 {
    cells.iter().map(|c| c.0).max().unwrap_or(0) + 1
}

fn shape_height(cells: &[(i8, i8)]) -> i8 {
    cells.iter().map(|c| c.1).max().unwrap_or(0) + 1
}

/// Rotate a shape 90 degrees clockwise and re-normalise it to the origin.
///
/// The I-piece is a special case: it is 4 cells long, so a horizontal rotation
/// would be 4 columns wide and can never fit a 3-lane panel. It is therefore kept
/// vertical and simply toggles between its two orientations, which are the only
/// ones that fit.
fn rotate(cells: &[(i8, i8)]) -> Vec<(i8, i8)> {
    let w = shape_width(cells);
    let h = shape_height(cells);
    if w == 1 && h == 4 {
        // Vertical I stays vertical.
        return cells.to_vec();
    }
    if w == 4 && h == 1 {
        // Horizontal I (should not occur) is forced back to vertical.
        return vec![(0, 0), (0, 1), (0, 2), (0, 3)];
    }

    let top = cells.iter().map(|c| c.1).max().unwrap_or(0);
    let mut out: Vec<(i8, i8)> = cells.iter().map(|&(x, y)| (top - y, x)).collect();

    let min_x = out.iter().map(|c| c.0).min().unwrap_or(0);
    let min_y = out.iter().map(|c| c.1).min().unwrap_or(0);
    for c in out.iter_mut() {
        c.0 -= min_x;
        c.1 -= min_y;
    }
    out
}

/// What the active piece is doing right now.
///
/// The previous implementation skipped straight to the resting position, so the
/// panel showed a new shape appearing at the bottom mid-stack with no explanation
/// - which read as random shapes overlapping. Every move is now an animated phase
/// so the drop is legible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// Turning toward `target_rot` one quarter turn at a time.
    Rotating,
    /// Falling one row per gravity step.
    Falling,
    /// A line is complete: flashing before it disappears.
    Clearing,
}

pub struct TetrisGame {
    cols: usize,
    rows: usize,
    /// `board[row][col]`, row 0 is the BOTTOM of the fan.
    board: Vec<Vec<Option<u8>>>,
    /// Active piece: footprint, palette colour, position, and the orientation
    /// index it currently holds. `rot` is kept even while falling so the rotation
    /// animation can resume without re-deriving the orientation.
    active: Option<(Vec<(i8, i8)>, u8, i8, i8, usize)>,
    /// Orientation the AI has decided the piece should end up in.
    target_rot: usize,
    /// Column the AI decided the piece should land in. The piece slides toward it
    /// as it falls, one column per gravity step, so the AI's plan is visible
    /// instead of being discarded at spawn.
    target_x: i8,
    phase: Phase,
    phase_timer: f32,
    /// Accumulated flash time, used to blink the completed rows.
    clear_timer: f32,
    /// Exactly which rows completed and are now flashing. The blink must apply to
    /// these rows ONLY - deriving them from the falling piece made the whole piece
    /// flash instead of the finished line.
    clearing_rows: Vec<usize>,
    /// Rows removed, purely informational for the preview.
    lines_cleared: u32,
    rng: Rng,
    /// Seconds per gravity step, already divided by the user's speed multiplier.
    step_secs: f32,
}

impl TetrisGame {
    pub fn new(cols: usize, rows: usize) -> Self {
        let mut g = Self {
            cols,
            rows,
            board: Vec::new(),
            active: None,
            target_rot: 0,
            target_x: 0,
            phase: Phase::Falling,
            phase_timer: 0.0,
            clear_timer: 0.0,
            clearing_rows: Vec::new(),
            lines_cleared: 0,
            rng: Rng::new(0x1234_5677),
            step_secs: TETRIS_STEP_SECS,
        };
        g.reset();
        g
    }

    pub fn dims(&self) -> (usize, usize) {
        (self.cols, self.rows)
    }

    fn reset(&mut self) {
        self.board = vec![vec![None; self.cols]; self.rows];
        self.active = None;
        self.target_rot = 0;
        self.target_x = 0;
        self.phase = Phase::Falling;
        self.phase_timer = 0.0;
        self.clear_timer = 0.0;
        self.clearing_rows.clear();
        self.spawn();
    }

    fn collides(&self, cells: &[(i8, i8)], x: i8, y: i8) -> bool {
        for &(cx, cy) in cells {
            let (px, py) = (x + cx, y + cy);
            if px < 0 || py < 0 || px >= self.cols as i8 || py >= self.rows as i8 {
                return true;
            }
            if self.board[py as usize][px as usize].is_some() {
                return true;
            }
        }
        false
    }

    /// Which rows become full once `cells` is resting at `(x, y)`.
    ///
    /// These rows - and only these - are the ones that flash on clear.
    fn completed_rows(&self, cells: &[(i8, i8)], x: i8, y: i8) -> Vec<usize> {
        let mut rows = Vec::new();
        for row in 0..self.rows {
            let mut full = true;
            for col in 0..self.cols as i8 {
                let filled_here = cells
                    .iter()
                    .any(|&(cx, cy)| x + cx == col && y + cy == row as i8);
                if !filled_here && self.board[row][col as usize].is_none() {
                    full = false;
                    break;
                }
            }
            if full {
                rows.push(row);
            }
        }
        rows
    }

    /// Lowest y at which the shape rests in column `x`, or -1 if it cannot fit.
    fn drop_y(&self, cells: &[(i8, i8)], x: i8) -> i8 {
        let top = self.rows as i8 - shape_height(cells);
        if top < 0 {
            return -1;
        }
        let mut y = top;
        while y > 0 && !self.collides(cells, x, y - 1) {
            y -= 1;
        }
        if self.collides(cells, x, y) {
            return -1;
        }
        y
    }

    /// Score a hypothetical placement. Classic heuristics: reward completed lines,
    /// penalise height, buried holes and a jagged surface.
    fn evaluate(&self, cells: &[(i8, i8)], x: i8, y: i8) -> f32 {
        let mut grid = vec![vec![false; self.cols]; self.rows];
        for (r, row) in self.board.iter().enumerate() {
            for (c, v) in row.iter().enumerate() {
                grid[r][c] = v.is_some();
            }
        }
        for &(cx, cy) in cells {
            let (px, py) = ((x + cx) as usize, (y + cy) as usize);
            if py < self.rows && px < self.cols {
                grid[py][px] = true;
            }
        }

        // Remove completed rows and remember how many.
        let mut cleared = 0.0;
        let mut kept: Vec<Vec<bool>> = Vec::with_capacity(self.rows);
        for row in grid.into_iter() {
            if row.iter().all(|&c| c) {
                cleared += 1.0;
            } else {
                kept.push(row);
            }
        }
        while kept.len() < self.rows {
            kept.push(vec![false; self.cols]);
        }

        let mut heights = vec![0i32; self.cols];
        let mut holes = 0i32;
        for c in 0..self.cols {
            let mut found = false;
            for r in (0..self.rows).rev() {
                if kept[r][c] {
                    if !found {
                        heights[c] = (r + 1) as i32;
                        found = true;
                    }
                } else if found {
                    holes += 1;
                }
            }
        }

        let agg: i32 = heights.iter().sum();
        let bump: i32 = heights.windows(2).map(|w| (w[0] - w[1]).abs()).sum();
        let max_h = heights.iter().copied().max().unwrap_or(0);

        0.90 * cleared - 0.51 * agg as f32 - 0.36 * holes as f32 - 0.18 * bump as f32
            - 0.40 * max_h as f32
    }

    /// Choose the best rotation and column for a new piece.
    ///
    /// Returns `(quarter_turns, column)`: the rotation is reported as a count of
    /// clockwise quarter turns from the spawn orientation so the caller can
    /// animate the spin rather than snapping to the final shape.
    fn best_placement(&mut self, shape: &[(i8, i8)]) -> (usize, i8) {
        let mut best: Option<(f32, usize, i8)> = None;
        let mut rot = shape.to_vec();

        for turns in 0..4 {
            let w = shape_width(&rot);
            let max_x = (self.cols as i8 - w).max(0);
            for x in 0..=max_x {
                let y = self.drop_y(&rot, x);
                if y < 0 {
                    continue;
                }
                let score = self.evaluate(&rot, x, y);
                let better = match &best {
                    None => true,
                    Some((bs, _, _)) => score > *bs,
                };
                if better {
                    best = Some((score, turns, x));
                }
            }
            rot = rotate(&rot);
        }

        best.map(|(_, turns, x)| (turns, x))
            .unwrap_or((0, 0))
    }

    /// Bring in a new piece at the top of the board, in its SPAWN orientation.
    ///
    /// The AI's chosen rotation becomes `target_rot` and is applied visibly during
    /// the `Rotating` phase instead of being pre-applied here, which is what makes
    /// the spin readable on the panel.
    fn spawn(&mut self) {
        if self.cols == 0 || self.rows == 0 {
            return;
        }
        let idx = self.rng.below(SHAPES.len());
        let color = PAL_PIECE_BASE + idx as u8;

        // Find the target orientation and column, then place the piece at the top
        // in its un-rotated form. `best_placement` searches all four rotations, so
        // the orientation it wants is reported back as a count of quarter turns.
        let (target_rot, target_x) = self.best_placement(SHAPES[idx]);

        let cells = SHAPES[idx].to_vec();
        let w = shape_width(&cells);
        // Clamp into the board: the target column was chosen for the rotated
        // footprint, which can be a different width to the spawn footprint. The
        // piece slides the rest of the way during `Falling`.
        let x = target_x.clamp(0, (self.cols as i8 - w).max(0));
        let y = self.rows as i8 - shape_height(&cells);

        if y < 0 || self.collides(&cells, x, y) {
            // Stack reached the ceiling: clear the board and start again.
            self.reset();
            return;
        }

        self.active = Some((cells, color, x, y, 0));
        self.target_rot = target_rot;
        self.target_x = target_x;
        self.phase = if target_rot == 0 {
            Phase::Falling
        } else {
            Phase::Rotating
        };
        self.phase_timer = TETRIS_SPAWN_HOLD_SECS;
    }

    /// Advance the game by `dt` seconds of real time.
    ///
    /// The phase machine means the timer length depends on what is happening, so
    /// unlike snake this cannot be a simple accumulator over `step_secs`.
    pub fn step(&mut self, dt: f32) {
        if self.cols == 0 || self.rows == 0 {
            return;
        }
        self.phase_timer -= dt;

        match self.phase {
            Phase::Rotating => {
                if self.phase_timer <= 0.0 {
                    self.rotate_once();
                }
            }
            Phase::Falling => {
                if self.phase_timer <= 0.0 {
                    self.gravity();
                }
            }
            Phase::Clearing => {
                self.clear_timer += dt;
                if self.phase_timer <= 0.0 {
                    self.finish_clear();
                }
            }
        }
    }

    /// Apply one quarter turn of the rotation animation.
    fn rotate_once(&mut self) {
        let Some((cells, color, x, y, rot)) = self.active.clone() else {
            self.phase = Phase::Falling;
            return;
        };

        let next = rotate(&cells);
        let next_rot = rot + 1;

        // Rotating can make a piece taller (T/S/Z/J/L go from 2 rows to 3), and the
        // piece spawns flush against the ceiling. Without dropping it by the extra
        // height the rotated form would stick out of row `rows-1` and `collides`
        // would reject every candidate - rotation would silently never happen.
        let growth = (shape_height(&next) - shape_height(&cells)).max(0);
        let y = (y - growth).max(0);

        // Try to stay in the current column, then slide sideways - a wall kick.
        // If nothing fits, abandon the rotation rather than dropping the piece
        // into the stack.
        let w = shape_width(&next);
        let max_x = (self.cols as i8 - w).max(0);
        let candidates = [x, x - 1, x + 1, 0, max_x];
        for cx in candidates {
            let cx = cx.clamp(0, max_x);
            if !self.collides(&next, cx, y) {
                let still_turning = next_rot < self.target_rot;
                self.active = Some((next, color, cx, y, next_rot));
                self.phase = if still_turning {
                    Phase::Rotating
                } else {
                    Phase::Falling
                };
                self.phase_timer = if still_turning {
                    TETRIS_ROTATE_STEP_SECS
                } else {
                    self.step_secs
                };
                return;
            }
        }

        // No orientation fits here. Fall from wherever the piece already is.
        self.phase = Phase::Falling;
        self.phase_timer = self.step_secs;
    }

    /// Move one column toward the AI's chosen column, if that is legal.
    ///
    /// Without this the piece simply fell straight down from wherever it spawned,
    /// so the stack looked arbitrary - the AI's plan was computed and then thrown
    /// away.
    fn slide_toward_target(&mut self, cells: &[(i8, i8)], color: u8, x: i8, y: i8, rot: usize) -> i8 {
        let w = shape_width(cells);
        let max_x = (self.cols as i8 - w).max(0);
        let want = self.target_x.clamp(0, max_x);
        if want == x {
            return x;
        }
        let step = if want > x { 1 } else { -1 };
        let nx = (x + step).clamp(0, max_x);
        if nx != x && !self.collides(cells, nx, y) {
            self.active = Some((cells.to_vec(), color, nx, y, rot));
            return nx;
        }
        x
    }

    fn gravity(&mut self) {
        let Some((cells, color, x, y, rot)) = self.active.clone() else {
            self.spawn();
            return;
        };

        // Steer toward the target column first, so the horizontal movement reads as
        // the piece lining itself up rather than drifting at random.
        let x = self.slide_toward_target(&cells, color, x, y, rot);

        if y > 0 && !self.collides(&cells, x, y - 1) {
            self.active = Some((cells, color, x, y - 1, rot));
            self.phase_timer = self.step_secs;
            return;
        }

        // Resting on the floor or the stack. If it closes one or more lines, lock
        // it in place and flash exactly those rows before removing them.
        let completed = self.completed_rows(&cells, x, y);
        if !completed.is_empty() {
            // Locking now (rather than after the flash) means the completed rows
            // are real board cells, so the piece's own colour is part of the row
            // that flashes.
            self.lock_piece(cells, color, x, y);
            self.clearing_rows = completed;
            self.phase = Phase::Clearing;
            self.phase_timer = TETRIS_CLEAR_FLASH_SECS;
            self.clear_timer = 0.0;
            return;
        }

        self.lock_piece(cells, color, x, y);
        self.spawn();
    }

    /// Write a piece into the board permanently.
    fn lock_piece(&mut self, cells: Vec<(i8, i8)>, color: u8, x: i8, y: i8) {
        for &(cx, cy) in &cells {
            let (px, py) = ((x + cx) as usize, (y + cy) as usize);
            if py >= self.rows || px >= self.cols {
                self.reset();
                return;
            }
            self.board[py][px] = Some(color);
        }
        self.active = None;
    }

    /// Remove the flashed rows, then bring in the next piece.
    fn finish_clear(&mut self) {
        // The piece was already locked into the board when the flash started.
        let mut rows = self.clearing_rows.clone();
        rows.sort_unstable();
        rows.dedup();

        // Remove highest index first, so the lower indices stay valid as the
        // board shifts down.
        for &r in rows.iter().rev() {
            if r < self.board.len() && self.board[r].iter().all(|c| c.is_some()) {
                self.board.remove(r);
                self.lines_cleared += 1;
            }
        }
        while self.board.len() < self.rows {
            self.board.push(vec![None; self.cols]);
        }
        self.clearing_rows.clear();
        self.clear_timer = 0.0;
        // `spawn` sets the phase for the new piece, so there is nothing to reset
        // here.
        self.spawn();
    }

    /// True while a completed line is in the middle of its flash.
    fn is_flashing(&self) -> bool {
        self.phase == Phase::Clearing
    }

    /// Which rows the active piece occupies, whether it is settled or mid-fall.
    /// Used by the tests to assert the piece moves gradually.
    #[cfg(test)]
    fn active_rows(&self) -> Vec<usize> {
        match &self.active {
            Some((cells, _, _, y, _)) => {
                let mut rows: Vec<usize> = cells.iter().map(|&(_, cy)| (y + cy) as usize).collect();
                rows.sort_unstable();
                rows.dedup();
                rows
            }
            None => Vec::new(),
        }
    }

    /// Palette index for a panel cell. `row` is vertical, `col` is the lane.
    ///
    /// While lines are clearing, only the completed rows blink. The falling piece
    /// keeps its own colour throughout, so a clear never makes the whole shape
    /// flash.
    pub fn cell(&self, row: usize, col: usize) -> Option<u8> {
        if row >= self.rows || col >= self.cols {
            return None;
        }

        // Only the rows that actually completed blink. Deriving this from the
        // active piece (`active_rows`) made a tall piece flash its entire body,
        // which is the bug this replaced.
        if self.is_flashing() && self.clearing_rows.contains(&row) {
            let on = (self.clear_timer * TETRIS_CLEAR_BLINKS_PER_SEC) as i32 % 2 == 0;
            return Some(if on { PAL_CLEAR_A } else { PAL_CLEAR_B });
        }

        if let Some((cells, color, x, y, _)) = &self.active {
            for &(cx, cy) in cells {
                let (px, py) = (x + cx, y + cy);
                if px >= 0 && py >= 0 && px as usize == col && py as usize == row {
                    return Some(*color);
                }
            }
        }
        self.board[row][col]
    }
}

impl Board for TetrisGame {
    fn cell(&self, row: usize, col: usize) -> Option<u8> {
        TetrisGame::cell(self, row, col)
    }

    fn dims(&self) -> (usize, usize) {
        self.dims()
    }

    fn snapshot(&self) -> GameSnapshot {
        let (lanes, rows) = self.dims();
        GameSnapshot {
            game: GameKind::Tetris,
            lanes: (0..lanes)
                .map(|c| (0..rows).map(|r| self.cell(r, c)).collect())
                .collect(),
            cleared: self.lines_cleared,
            flashing: self.is_flashing(),
            clearing_rows: self.clearing_rows.clone(),
        }
    }
}

// ---------------------------------------------------------------- container

/// Holds both games so the engine owns a single piece of mutable state.
pub struct Games {
    pub snake: SnakeGame,
    pub tetris: TetrisGame,
    /// User's play-speed multiplier. 1.0 is the tuned default.
    speed: f32,
}

impl Games {
    /// `lanes` is the number of columns (3 on this fan); `leds_per_lane` is the
    /// number of rows running down its length (13). The names are spelled out
    /// because swapping them silently produces a 3-row board and a mostly dark
    /// panel, which is exactly what happened once.
    pub fn new(lanes: usize, leds_per_lane: usize) -> Self {
        Self {
            snake: SnakeGame::new(lanes, leds_per_lane),
            tetris: TetrisGame::new(lanes, leds_per_lane),
            speed: 1.0,
        }
    }

    /// Rebuild the games if the panel shape changed, since a resized board would
    /// otherwise leave the snake or stack outside the visible area.
    pub fn sync_shape(&mut self, lanes: usize, leds_per_lane: usize) {
        if self.snake.dims() != (lanes, leds_per_lane) {
            self.snake = SnakeGame::new(lanes, leds_per_lane);
            self.snake.step_secs = self.snake_step_secs();
        }
        if self.tetris.dims() != (lanes, leds_per_lane) {
            self.tetris = TetrisGame::new(lanes, leds_per_lane);
            self.tetris.step_secs = self.tetris_step_secs();
        }
    }

    /// Apply the user's speed multiplier (1.0 = the tuned default) and start the
    /// games from a clean board.
    ///
    /// Restarting is deliberate: changing the speed mid-game would otherwise alter
    /// the timing of a piece that is already falling, which looks like a glitch.
    pub fn set_speed(&mut self, speed: f32) {
        let speed = speed.clamp(0.25, 4.0);
        if (speed - self.speed).abs() < f32::EPSILON {
            return;
        }
        self.speed = speed;
        self.snake = SnakeGame::new(self.snake.dims().0, self.snake.dims().1);
        self.tetris = TetrisGame::new(self.tetris.dims().0, self.tetris.dims().1);
        self.snake.step_secs = self.snake_step_secs();
        self.tetris.step_secs = self.tetris_step_secs();
    }

    fn snake_step_secs(&self) -> f32 {
        (SNAKE_STEP_SECS / self.speed).clamp(0.04, 1.5)
    }

    fn tetris_step_secs(&self) -> f32 {
        (TETRIS_STEP_SECS / self.speed).clamp(0.05, 2.0)
    }

    /// Snapshot whichever game is showing, for the UI preview.
    pub fn snapshot(&self, game: GameKind) -> GameSnapshot {
        match game {
            GameKind::Tetris => self.tetris.snapshot(),
            GameKind::Snake => self.snake.snapshot(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snake_starts_inside_the_board() {
        let g = SnakeGame::new(3, 13);
        for &(x, y) in &g.body {
            assert!(x < 3 && y < 13, "snake spawned outside the panel");
        }
    }

    #[test]
    fn snake_survives_many_steps() {
        // The AI must not instantly kill itself; a reset means it started over,
        // which is fine, but it should still be alive at the end.
        let mut g = SnakeGame::new(3, 13);
        for _ in 0..2000 {
            g.step(0.22);
        }
        assert!(!g.body.is_empty(), "snake died and failed to respawn");
    }

    #[test]
    fn snake_never_leaves_the_board() {
        let mut g = SnakeGame::new(3, 13);
        for _ in 0..4000 {
            g.step(0.22);
            for &(x, y) in &g.body {
                assert!(x < 3 && y < 13, "snake moved out of bounds at ({x},{y})");
            }
        }
    }

    #[test]
    fn snake_does_not_overlap_itself() {
        let mut g = SnakeGame::new(3, 13);
        for _ in 0..2000 {
            g.step(0.22);
            let mut seen: Vec<(usize, usize)> = g.body.iter().copied().collect();
            let len = seen.len();
            seen.sort_unstable();
            seen.dedup();
            assert_eq!(seen.len(), len, "snake overlapped itself");
        }
    }

    #[test]
    fn snake_uses_the_full_length_of_the_fan() {
        // On this fan the board is 3 lanes x 13 rows. If the games were built
        // with those two swapped, the snake would be trapped in the bottom three
        // rows and everything above row 2 would render black.
        let mut g = SnakeGame::new(3, 13);
        let mut highest = 0;
        for _ in 0..600 {
            g.step(SNAKE_STEP_SECS);
            for &(_, y) in &g.body {
                highest = highest.max(y);
            }
        }
        assert!(
            highest >= 10,
            "snake only reached row {highest} of 13; the board is probably transposed"
        );
    }

    #[test]
    fn tetris_uses_the_full_length_of_the_fan() {
        // The same transposition guard as the snake test, for the stack: pieces
        // must be able to settle high up the fan, not just in the first 3 rows.
        let mut g = TetrisGame::new(3, 13);
        let mut highest = 0;
        for _ in 0..2400 {
            g.step(TETRIS_STEP_SECS / 4.0);
            for row in g.active_rows() {
                highest = highest.max(row);
            }
        }
        assert!(
            highest >= 8,
            "tetris pieces only reached row {highest} of 13; the board is probably transposed"
        );
    }

    #[test]
    fn games_use_lanes_as_columns() {
        let g = Games::new(3, 13);
        assert_eq!(g.snake.dims(), (3, 13));
        assert_eq!(g.tetris.dims(), (3, 13));
        let (cols, rows) = g.snake.dims();
        assert!(rows > cols, "the board must be taller than it is wide");
    }

    #[test]
    fn all_tetrominoes_fit_three_columns_after_rotation() {
        for shape in SHAPES {
            let mut rot = shape.to_vec();
            for _ in 0..4 {
                assert!(
                    shape_width(&rot) <= 3,
                    "a rotation is {} wide, which cannot fit 3 lanes",
                    shape_width(&rot)
                );
                assert!(
                    shape_height(&rot) <= 4,
                    "a rotation is {} tall, which cannot fit 13 rows",
                    shape_height(&rot)
                );
                rot = rotate(&rot);
            }
        }
    }

    #[test]
    fn horizontal_i_piece_is_rejected() {
        // The I-piece is the only 4-long shape, so a horizontal orientation can
        // never fit. `rotate` must refuse to produce one.
        let horizontal = [(0i8, 0i8), (1, 0), (2, 0), (3, 0)];
        let fixed = rotate(&horizontal);
        assert!(
            shape_width(&fixed) <= 3,
            "rotate produced a {}-wide shape",
            shape_width(&fixed)
        );
    }

    #[test]
    fn rotation_preserves_cell_count_and_is_stable() {
        for shape in SHAPES {
            let mut rot = shape.to_vec();
            for _ in 0..4 {
                assert_eq!(rot.len(), shape.len(), "rotation changed the cell count");
                rot = rotate(&rot);
            }
            // Four rotations return to the original footprint.
            let mut sorted_a: Vec<(i8, i8)> = rot.clone();
            let mut sorted_b: Vec<(i8, i8)> = shape.to_vec();
            sorted_a.sort_unstable();
            sorted_b.sort_unstable();
            assert_eq!(sorted_a, sorted_b, "four rotations should be identity");
        }
    }

    #[test]
    fn tetris_never_panics_and_keeps_playing() {
        let mut g = TetrisGame::new(3, 13);
        for _ in 0..4000 {
            // Feed the timer in small slices so the phase machine is exercised the
            // way the engine drives it, rather than in one giant catch-up step.
            for _ in 0..8 {
                g.step(TETRIS_STEP_SECS / 8.0);
            }
            assert!(!g.board.is_empty(), "board collapsed");
            assert_eq!(g.board.len(), 13, "board height drifted");
            for row in &g.board {
                assert_eq!(row.len(), 3, "board width drifted");
            }
        }
    }

    #[test]
    fn tetris_cells_stay_in_bounds() {
        let mut g = TetrisGame::new(3, 13);
        for _ in 0..2000 {
            for _ in 0..8 {
                g.step(TETRIS_STEP_SECS / 8.0);
            }
            for row in 0..13 {
                for col in 0..3 {
                    // Any value returned must be a real palette colour.
                    if let Some(idx) = g.cell(row, col) {
                        assert!(idx <= PAL_MAX, "unknown palette index {idx}");
                    }
                }
            }
        }
    }

    #[test]
    fn rotating_pieces_never_overflow_the_ceiling() {
        // T/S/Z/J/L grow from 2 rows to 3 when rotated. The piece spawns flush
        // against the ceiling, so without a downward kick the rotated form sticks
        // out of the top row and every candidate position fails `collides` -
        // rotation silently never happens for 5 of the 7 shapes.
        for (i, shape) in SHAPES.iter().enumerate() {
            let mut g = TetrisGame::new(3, 13);
            // Force this shape into play at the top of the board, aimed at a
            // rotation that grows it.
            let top = 13 - shape_height(shape);
            g.active = Some((shape.to_vec(), PAL_PIECE_BASE + i as u8, 0, top, 0));
            g.target_rot = 1;
            g.phase = Phase::Rotating;
            g.phase_timer = 0.0;

            g.step(TETRIS_ROTATE_STEP_SECS);

            let Some((cells, _, x, y, rot)) = &g.active else {
                panic!("shape {i} vanished during rotation");
            };
            assert!(
                !g.collides(cells, *x, *y),
                "shape {i} is in an illegal position after rotating"
            );
            for &(cx, cy) in cells {
                assert!(
                    (x + cx) >= 0 && (x + cx) < 3,
                    "shape {i} left the board sideways at column {}",
                    x + cx
                );
                assert!(
                    (y + cy) >= 0 && (y + cy) < 13,
                    "shape {i} overflowed the ceiling: row {} of 13",
                    y + cy
                );
            }
            // The rotation must actually have advanced, not been abandoned.
            if shape_height(shape) == 2 {
                assert_eq!(*rot, 1, "shape {i} failed to rotate at all");
            }
        }
    }

    #[test]
    fn tetris_piece_falls_one_row_at_a_time() {        // The piece must descend gradually; the old implementation teleported it
        // straight to the resting row, which is what looked like random shapes
        // appearing on top of the stack.
        let mut g = TetrisGame::new(3, 13);
        let start_rows = g.active_rows();
        let start_top = *start_rows.iter().max().unwrap_or(&0);

        // Step through two gravity intervals in small slices.
        for _ in 0..8 {
            g.step(TETRIS_STEP_SECS / 4.0);
        }

        let now_top = *g.active_rows().iter().max().unwrap_or(&0);
        assert!(
            now_top < start_top,
            "piece did not fall: started at row {start_top}, still at row {now_top}"
        );
        assert!(
            now_top + 2 >= start_top,
            "piece fell {start_top} -> {now_top} in one interval, which is a teleport"
        );
    }

    #[test]
    fn tetris_never_overlaps_settled_cells() {
        // Two cells may never occupy the same position. This is the invariant the
        // "random shapes overlapping each other" complaint was about.
        let mut g = TetrisGame::new(3, 13);
        for _ in 0..3000 {
            for _ in 0..4 {
                g.step(TETRIS_STEP_SECS / 4.0);
            }
            // The active piece must not collide with the settled board.
            if let Some((cells, _, x, y, _)) = &g.active {
                for &(cx, cy) in cells {
                    let (px, py) = (x + cx, y + cy);
                    assert!(px >= 0 && py >= 0, "active cell out of bounds");
                    assert!(
                        (px as usize) < 3 && (py as usize) < 13,
                        "active cell out of bounds"
                    );
                    if !g.is_flashing() {
                        assert!(
                            g.board[py as usize][px as usize].is_none(),
                            "active piece overlaps a settled cell at ({px},{py})"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn tetris_clears_lines_and_pauses_while_flashing() {
        // Clear the board, then fill an entire row by hand so the next lock
        // completes it.
        let mut g = TetrisGame::new(3, 13);
        for _ in 0..6000 {
            for _ in 0..4 {
                g.step(TETRIS_STEP_SECS / 4.0);
            }
            if g.is_flashing() {
                // While flashing, the falling piece must not move: the phase timer
                // is what pauses gravity.
                let before = g.active.clone();
                g.step(TETRIS_CLEAR_FLASH_SECS / 4.0);
                if g.is_flashing() {
                    match (&before, &g.active) {
                        (Some((_, _, x1, y1, _)), Some((_, _, x2, y2, _))) => {
                            assert_eq!((x1, y1), (x2, y2), "piece moved during the clear flash");
                        }
                        _ => {}
                    }
                }
            }
        }
        assert!(
            g.lines_cleared > 0,
            "no line was ever cleared in 6000 steps, so the flash never triggers"
        );
    }

    #[test]
    fn only_completed_rows_blink_not_the_whole_piece() {
        // The reported symptom: the WHOLE piece blinked instead of just the
        // finished line. The cause was deriving the flashing rows from the active
        // piece (`active_rows`), so a tall piece lit up entirely.
        //
        // A vertical I-piece is the clearest reproduction: it is 4 rows tall, and
        // it completes row 0, so rows 0..3 must behave differently.
        let mut g = TetrisGame::new(3, 13);
        let piece_color = PAL_PIECE_BASE; // the I-piece colour
        // Row 0 is already full except the middle lane.
        g.board[0][0] = Some(piece_color);
        g.board[0][2] = Some(piece_color);

        // Vertical I in the middle lane, resting on the floor: rows 0..=3.
        let cells = vec![(0i8, 0i8), (0, 1), (0, 2), (0, 3)];
        g.active = Some((cells, piece_color, 1, 0, 0));
        g.phase = Phase::Falling;
        g.phase_timer = 0.0;

        // Gravity with y == 0 locks the piece and detects the completed row.
        g.step(TETRIS_STEP_SECS);

        assert!(g.is_flashing(), "row 0 completed, so a clear must be running");
        assert_eq!(
            g.clearing_rows,
            vec![0],
            "only row 0 is complete; rows 1-3 must not be marked for clearing"
        );

        // Row 0 blinks.
        let row0 = g.cell(0, 1).unwrap();
        assert!(
            row0 == PAL_CLEAR_A || row0 == PAL_CLEAR_B,
            "row 0 should be flashing, got {row0}"
        );

        // Rows 1..3 are part of the same piece but are NOT complete, so they must
        // keep the piece's colour rather than blinking.
        for row in 1..=3 {
            assert_eq!(
                g.cell(row, 1),
                Some(piece_color),
                "row {row} is part of the piece but is not complete - it must not blink"
            );
        }
    }

    #[test]
    fn clearing_removes_exactly_the_completed_rows() {
        let mut g = TetrisGame::new(3, 13);
        let piece_color = PAL_PIECE_BASE;
        // Row 0 completes; row 1 keeps a single settled cell.
        g.board[0][0] = Some(piece_color);
        g.board[0][2] = Some(piece_color);
        g.board[1][0] = Some(piece_color);

        let cells = vec![(0i8, 0i8), (0, 1), (0, 2), (0, 3)];
        g.active = Some((cells, piece_color, 1, 0, 0));
        g.phase = Phase::Falling;
        g.phase_timer = 0.0;
        g.step(TETRIS_STEP_SECS);
        assert!(g.is_flashing());

        // Run past the flash to trigger the actual removal.
        g.step(TETRIS_CLEAR_FLASH_SECS + 0.01);

        assert_eq!(g.lines_cleared, 1, "exactly one line should have been removed");
        assert!(!g.is_flashing(), "the clear should be finished");
        assert!(g.clearing_rows.is_empty(), "clearing rows should be reset");
        // Row 1's surviving cell must have shifted down into row 0.
        assert_eq!(
            g.board[0][0],
            Some(piece_color),
            "the settled cell above the cleared row should drop down"
        );
    }

    #[test]
    fn snake_snapshot_tracks_the_body_every_step() {
        // The UI preview renders `snapshot()`. For the preview to show the real
        // state it must change on every move and match `cell()` exactly.
        let mut g = SnakeGame::new(3, 13);
        let mut previous: Option<Vec<Vec<Option<u8>>>> = None;
        let mut changes = 0;

        for _ in 0..60 {
            g.step(SNAKE_STEP_SECS);
            let snap = g.snapshot();

            // The snapshot must agree with `cell()` cell-for-cell.
            for (lane, column) in snap.lanes.iter().enumerate() {
                for (row, value) in column.iter().enumerate() {
                    assert_eq!(
                        *value,
                        g.cell(row, lane),
                        "snapshot disagrees with cell() at lane {lane} row {row}"
                    );
                }
            }

            // Lane-major shape must match the board.
            assert_eq!(snap.lanes.len(), 3, "snapshot must have one entry per lane");
            for column in &snap.lanes {
                assert_eq!(column.len(), 13, "snapshot lane must be 13 rows long");
            }

            let lit: usize = snap.lanes.iter().flatten().filter(|c| c.is_some()).count();
            assert!(
                lit >= 3,
                "the snake should always have at least a head, body and food lit"
            );

            if let Some(prev) = &previous {
                if *prev != snap.lanes {
                    changes += 1;
                }
            }
            previous = Some(snap.lanes);
        }

        assert!(
            changes >= 50,
            "the snapshot only changed {changes} times in 60 moves - the preview would look frozen"
        );
    }

    #[test]
    fn snapshot_is_tagged_with_its_game() {
        let g = Games::new(3, 13);
        assert_eq!(g.snapshot(GameKind::Snake).game, GameKind::Snake);
        assert_eq!(g.snapshot(GameKind::Tetris).game, GameKind::Tetris);
    }

    #[test]
    fn speed_multiplier_scales_the_step_interval() {
        let mut slow = Games::new(3, 13);
        slow.set_speed(0.5);
        let mut fast = Games::new(3, 13);
        fast.set_speed(4.0);
        assert!(
            fast.snake.step_secs < slow.snake.step_secs,
            "faster speed must mean a shorter interval"
        );
        assert!(
            fast.tetris.step_secs < slow.tetris.step_secs,
            "faster speed must mean a shorter interval"
        );
    }

    #[test]
    fn tetris_pieces_actually_rotate_into_play() {
        // Rotation was silently impossible before the ceiling-kick fix, so no
        // piece ever appeared rotated and the stack looked arbitrary.
        let mut g = TetrisGame::new(3, 13);
        let mut rotated_any = false;
        for _ in 0..4000 {
            g.step(TETRIS_STEP_SECS / 4.0);
            if let Some((_, _, _, _, rot)) = &g.active {
                if *rot > 0 {
                    rotated_any = true;
                }
            }
        }
        assert!(rotated_any, "no piece ever rotated in 4000 steps");
    }

    #[test]
    fn pieces_steer_toward_the_chosen_column() {
        // The AI picks a column; the piece must move toward it rather than falling
        // straight down from wherever it spawned.
        let mut g = TetrisGame::new(3, 13);
        let mut moved = false;
        for _ in 0..200 {
            let before = g.active.as_ref().map(|a| a.2);
            let target = g.target_x;
            g.step(TETRIS_STEP_SECS);
            if let (Some(x0), Some(a)) = (before, g.active.as_ref()) {
                if a.2 != x0 {
                    moved = true;
                }
                // Horizontal movement must always reduce the distance to target.
                if target != x0 {
                    assert!(
                        (a.2 - target).abs() <= (x0 - target).abs(),
                        "piece moved away from its target column"
                    );
                }
            }
        }
        assert!(moved, "no piece ever moved sideways");
    }
}
