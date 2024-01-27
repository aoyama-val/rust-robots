use rand::prelude::*;
use std::{ops, time};

pub const FPS: i32 = 30;
pub const FIELD_W: i32 = 36;
pub const FIELD_H: i32 = 36;
pub const CELL_W: i32 = 16;
pub const CELL_H: i32 = 16;
pub const ROBOT_COUNT_BASE: i32 = 11;
pub const ROBOT_COUNT_PER_LEVEL: i32 = 5;
pub const ROBOT_COUNT_MAX: i32 = FIELD_W * FIELD_H / 4;

// $varの値が
//   > 0 : ウェイト中
//  == 0 : ブロック実行
//   < 0 : ブロック実行せず、ウェイトも減らさない
macro_rules! wait {
    ($var:expr, $block:block) => {
        if $var > 0 {
            $var -= 1;
        }
        if $var == 0 {
            $block
        }
    };
}
pub(crate) use wait;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Command {
    None,
    Left,
    Right,
    Down,
    Up,
    UpLeft,
    UpRight,
    DownLeft,
    DownRight,
    Teleport,
    Wait,
    NextLevel,
}

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
pub struct Vec2 {
    pub x: i32,
    pub y: i32,
}

impl ops::Add<Vec2> for Vec2 {
    type Output = Vec2;

    fn add(self, _rhs: Vec2) -> Vec2 {
        Vec2 {
            x: self.x + _rhs.x,
            y: self.y + _rhs.y,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum Direction {
    Left,
    Right,
    Down,
    Up,
    UpLeft,
    UpRight,
    DownLeft,
    DownRight,
}

impl Direction {
    pub fn to_vec2(&self) -> Vec2 {
        match self {
            Direction::Left => Vec2 { x: -1, y: 0 },
            Direction::Right => Vec2 { x: 1, y: 0 },
            Direction::Down => Vec2 { x: 0, y: 1 },
            Direction::Up => Vec2 { x: 0, y: -1 },
            Direction::UpLeft => Vec2 { x: -1, y: -1 },
            Direction::UpRight => Vec2 { x: 1, y: -1 },
            Direction::DownLeft => Vec2 { x: -1, y: 1 },
            Direction::DownRight => Vec2 { x: 1, y: 1 },
        }
    }
}

#[derive(Debug, Default)]
pub struct Player {
    pub pos: Vec2,
}

#[derive(Debug)]
pub struct Robot {
    pub pos: Vec2,
    pub exist: bool,
}

#[derive(Debug)]
pub struct Junk {
    pub pos: Vec2,
}

#[derive(Debug)]
pub struct LaserCannon {
    pub pos: Vec2,
    pub turn: i32,
    pub direction: Direction,
}

#[derive(Debug)]
pub struct Game {
    pub rng: Option<StdRng>,
    pub frame: i32,
    pub requested_sounds: Vec<&'static str>,
    pub is_over: bool,
    pub is_clear: bool,
    pub level: i32,
    pub initial_robot_count: i32,
    pub player: Player,
    pub robots: Vec<Robot>,
    pub junks: Vec<Junk>,
    pub laser_cannon: LaserCannon,
}

impl Game {
    pub fn new() -> Self {
        let now = time::SystemTime::now();
        let timestamp = now
            .duration_since(time::UNIX_EPOCH)
            .expect("SystemTime before UNIX EPOCH!")
            .as_secs();
        let rng = StdRng::seed_from_u64(timestamp);
        println!("random seed = {}", timestamp);

        let mut game = Game {
            rng: Some(rng),
            frame: -1,
            requested_sounds: Vec::new(),
            is_over: false,
            is_clear: false,
            level: 0,
            initial_robot_count: 0,
            player: Player::default(),
            robots: Vec::new(),
            junks: Vec::new(),
            laser_cannon: LaserCannon {
                pos: Vec2::default(),
                turn: 0,
                direction: Direction::Up,
            },
        };

        game.next_level();

        game
    }

    pub fn next_level(&mut self) {
        self.is_over = false;
        self.is_clear = false;
        self.level += 1;
        self.player.pos.x = FIELD_W / 2;
        self.player.pos.y = FIELD_H / 2;
        self.robots = Vec::new();
        self.junks = Vec::new();
        self.spawn_robots();
        self.set_laser_cannon();
    }

    pub fn spawn_robots(&mut self) {
        let robot_count = clamp(
            0,
            ROBOT_COUNT_BASE + self.level * ROBOT_COUNT_PER_LEVEL,
            ROBOT_COUNT_MAX,
        );
        self.initial_robot_count = robot_count;
        while (self.robots.len() as i32) < robot_count {
            let x = self.rng.as_mut().unwrap().gen_range(0..FIELD_W);
            let y = self.rng.as_mut().unwrap().gen_range(0..FIELD_H);
            let mut should_add = true;
            if x.abs_diff(self.player.pos.x) <= 1 && y.abs_diff(self.player.pos.y) <= 1 {
                should_add = false;
            }
            for robot in &self.robots {
                if robot.pos.x == x && robot.pos.y == y {
                    should_add = false;
                    break;
                }
            }
            if should_add {
                self.robots.push(Robot {
                    pos: Vec2 { x, y },
                    exist: true,
                })
            }
        }
    }

    pub fn set_laser_cannon(&mut self) {
        let quarter_w = FIELD_W / 4;
        let quarter_h = FIELD_H / 4;
        let pos: Vec2 = Vec2 {
            x: FIELD_W / 2 + self.rng.as_mut().unwrap().gen_range(-quarter_w..=quarter_w),
            y: FIELD_H / 2 + self.rng.as_mut().unwrap().gen_range(-quarter_h..quarter_h),
        };
        self.laser_cannon = LaserCannon {
            pos: pos,
            turn: 0,
            direction: Direction::Right,
        };
    }

    pub fn update(&mut self, command: Command) {
        self.frame += 1;

        if self.is_over {
            return;
        }

        if self.is_clear {
            if command == Command::NextLevel {
                self.next_level();
            }
            return;
        }

        match command {
            Command::None => return,
            Command::Left => self.move_player(Direction::Left),
            Command::Right => self.move_player(Direction::Right),
            Command::Down => self.move_player(Direction::Down),
            Command::Up => self.move_player(Direction::Up),
            Command::UpLeft => self.move_player(Direction::UpLeft),
            Command::UpRight => self.move_player(Direction::UpRight),
            Command::DownLeft => self.move_player(Direction::DownLeft),
            Command::DownRight => self.move_player(Direction::DownRight),
            Command::Teleport => self.teleport(),
            Command::Wait => {}
            Command::NextLevel => return,
        }

        self.move_robots();

        // ロボットの衝突より前に実行。そうしないと、2体以上のロボットが同時にプレイヤーに接触したときゲームオーバーにならない
        self.check_gameover();

        self.check_robots_collision();

        self.check_clear();

        self.robots.retain(|x| x.exist);
    }

    pub fn move_player(&mut self, direction: Direction) {
        let v = direction.to_vec2();
        let x = self.player.pos.x + v.x;
        let y = self.player.pos.y + v.y;
        if 0 <= x && x < FIELD_W && 0 <= y && y < FIELD_H {
            if self.is_junk(x, y) {
                self.requested_sounds.push("ng.wav");
                return;
            }
            self.player.pos.x = x;
            self.player.pos.y = y;
        }
    }

    pub fn is_junk(&self, x: i32, y: i32) -> bool {
        self.junks.iter().any(|j| j.pos.x == x && j.pos.y == y)
    }

    pub fn teleport(&mut self) {
        let x = self.rng.as_mut().unwrap().gen_range(0..FIELD_W);
        let y = self.rng.as_mut().unwrap().gen_range(0..FIELD_H);
        self.player.pos.x = x;
        self.player.pos.y = y;
        self.requested_sounds.push("shoot.wav");
    }

    pub fn move_robots(&mut self) {
        for robot in &mut self.robots {
            let vx: i32 = (self.player.pos.x - robot.pos.x).signum();
            let vy: i32 = (self.player.pos.y - robot.pos.y).signum();
            robot.pos.x = clamp(0, robot.pos.x + vx, FIELD_W - 1);
            robot.pos.y = clamp(0, robot.pos.y + vy, FIELD_H - 1);
        }
    }

    pub fn check_robots_collision(&mut self) {
        for i in 0..self.robots.len() {
            if self.robots[i].exist {
                if self.is_junk(self.robots[i].pos.x, self.robots[i].pos.y) {
                    self.robots[i].exist = false;
                    self.requested_sounds.push("hit.wav");
                }
                for j in (i + 1)..self.robots.len() {
                    if self.robots[i].pos == self.robots[j].pos {
                        self.junks.push(Junk {
                            pos: self.robots[i].pos,
                        });
                        self.robots[i].exist = false;
                        self.robots[j].exist = false;
                        self.requested_sounds.push("hit.wav");
                    }
                }
            }
        }
    }

    pub fn check_gameover(&mut self) {
        for robot in &self.robots {
            if robot.pos == self.player.pos {
                self.is_over = true;
                self.requested_sounds.push("crash.wav");
                break;
            }
        }
    }

    pub fn check_clear(&mut self) {
        if self
            .robots
            .iter()
            .filter(|x| x.exist)
            .collect::<Vec<_>>()
            .is_empty()
        {
            self.is_clear = true;
            self.requested_sounds.push("bravo.wav");
        }
    }
}

fn clamp<T: PartialOrd>(min: T, value: T, max: T) -> T {
    if value < min {
        return min;
    }
    if value > max {
        return max;
    }
    value
}
