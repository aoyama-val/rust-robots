use rand::prelude::*;
use std::time;

pub const FPS: i32 = 30;
pub const FIELD_W: usize = 36;
pub const FIELD_H: usize = 36;
pub const CELL_W: i32 = 16;
pub const CELL_H: i32 = 16;
pub const ROBOT_COUNT: usize = 11;
pub const ROBOT_COUNT_PER_LEVEL: usize = 5;
pub const ROBOT_COUNT_MAX: usize = FIELD_W * FIELD_H / 4;

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

impl Command {
    pub fn from_str(str: &str) -> Command {
        match str {
            "None" => Command::None,
            "Left" => Command::Left,
            "Right" => Command::Right,
            "Up" => Command::Up,
            "Down" => Command::Down,
            "UpLeft" => Command::UpLeft,
            "UpRight" => Command::UpRight,
            "DownLeft" => Command::DownLeft,
            "DownRight" => Command::DownRight,
            "Teleport" => Command::Teleport,
            "Wait" => Command::Wait,
            _ => Command::None,
        }
    }
}

pub struct Vec2 {
    x: i32,
    y: i32,
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

#[derive(Debug)]
pub struct Robot {
    pub x: usize,
    pub y: usize,
    pub exist: bool,
}

#[derive(Debug)]
pub struct Junk {
    pub x: usize,
    pub y: usize,
}

#[derive(Debug)]
pub struct Game {
    pub rng: Option<StdRng>,
    pub is_over: bool,
    pub is_clear: bool,
    pub frame: i32,
    pub requested_sounds: Vec<&'static str>,
    pub player_x: usize,
    pub player_y: usize,
    pub robots: Vec<Robot>,
    pub junks: Vec<Junk>,
    pub level: i32,
    pub initial_robot_count: usize,
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
            is_over: false,
            is_clear: false,
            requested_sounds: Vec::new(),
            player_x: 0,
            player_y: 0,
            robots: Vec::new(),
            junks: Vec::new(),
            level: 0,
            initial_robot_count: 0,
        };

        game.next_level();

        game
    }

    pub fn next_level(&mut self) {
        self.level += 1;
        self.player_x = FIELD_W / 2;
        self.player_y = FIELD_H / 2;
        self.is_clear = false;
        self.robots = Vec::new();
        self.junks = Vec::new();
        self.spawn_robots();
    }

    pub fn spawn_robots(&mut self) {
        let robot_count = clamp(
            0,
            ROBOT_COUNT + self.level as usize * ROBOT_COUNT_PER_LEVEL,
            ROBOT_COUNT_MAX,
        );
        self.initial_robot_count = robot_count;
        while self.robots.len() < robot_count {
            let x = self.rng.as_mut().unwrap().gen_range(0..FIELD_W);
            let y = self.rng.as_mut().unwrap().gen_range(0..FIELD_H);
            let mut should_add = true;
            if x.abs_diff(self.player_x) <= 1 && y.abs_diff(self.player_y) <= 1 {
                should_add = false;
            }
            for robot in &self.robots {
                if robot.x == x && robot.y == y {
                    should_add = false;
                    break;
                }
            }
            if should_add {
                self.robots.push(Robot { x, y, exist: true })
            }
        }
    }

    pub fn update(&mut self, mut command: Command) {
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

        self.check_robots_collision();

        self.check_gameover();
        self.check_clear();

        self.robots.retain(|x| x.exist);
    }

    pub fn move_player(&mut self, direction: Direction) {
        let v = direction.to_vec2();
        let x = self.player_x as i32 + v.x;
        let y = self.player_y as i32 + v.y;
        if 0 <= x && x < FIELD_W as i32 && 0 <= y && y < FIELD_H as i32 {
            if self.is_junk(x as usize, y as usize) {
                self.requested_sounds.push("ng.wav");
                return;
            }
            self.player_x = x as usize;
            self.player_y = y as usize;
        }
    }

    pub fn is_junk(&self, x: usize, y: usize) -> bool {
        self.junks.iter().any(|j| j.x == x && j.y == y)
    }

    pub fn teleport(&mut self) {
        let x = self.rng.as_mut().unwrap().gen_range(0..FIELD_W);
        let y = self.rng.as_mut().unwrap().gen_range(0..FIELD_H);
        self.player_x = x;
        self.player_y = y;
        self.requested_sounds.push("shoot.wav");
    }

    pub fn move_robots(&mut self) {
        for robot in &mut self.robots {
            let vx: i32 = if self.player_x > robot.x {
                1
            } else if self.player_x < robot.x {
                -1
            } else {
                0
            };
            let vy: i32 = if self.player_y > robot.y {
                1
            } else if self.player_y < robot.y {
                -1
            } else {
                0
            };
            robot.x = clamp(0, robot.x as i32 + vx, FIELD_W as i32 - 1) as usize;
            robot.y = clamp(0, robot.y as i32 + vy, FIELD_H as i32 - 1) as usize;
        }
    }

    pub fn check_robots_collision(&mut self) {
        for i in 0..self.robots.len() {
            if self.robots[i].exist {
                if self.is_junk(self.robots[i].x, self.robots[i].y) {
                    self.robots[i].exist = false;
                    self.requested_sounds.push("hit.wav");
                }
                for j in (i + 1)..self.robots.len() {
                    if self.robots[i].x == self.robots[j].x && self.robots[i].y == self.robots[j].y
                    {
                        self.junks.push(Junk {
                            x: self.robots[i].x,
                            y: self.robots[i].y,
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
            if robot.x == self.player_x && robot.y == self.player_y {
                self.is_over = true;
                self.requested_sounds.push("crash.wav");
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
