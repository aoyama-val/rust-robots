use rand::prelude::*;
use std::{fs::File, io::Write, time};

pub const FPS: i32 = 30;
pub const FIELD_W: usize = 33;
pub const FIELD_H: usize = 33;
pub const CELL_W: i32 = 16;
pub const CELL_H: i32 = 16;
pub const EMPTY: i32 = 0;
pub const JUNK: i32 = 1;
pub const ROBOT_COUNT: usize = 11;
pub const ROBOT_COUNT_PER_LEVEL: usize = 5;
pub const ENERGY_MAX: f32 = 100.0;
pub const ENERGY_INCREASE_SPEED: f32 = 0.1;
pub const TELEPORT_ENERGY: f32 = 25.0;

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
pub struct Game {
    pub rng: Option<StdRng>,
    pub is_over: bool,
    pub is_clear: bool,
    pub is_debug: bool,
    pub frame: i32,
    pub requested_sounds: Vec<&'static str>,
    pub commands: Vec<Command>,    // リプレイデータから読み込んだコマンド
    pub command_log: Option<File>, // コマンドログ
    pub replay_loaded: bool,
    pub field: [[i32; FIELD_W]; FIELD_H],
    pub player_x: usize,
    pub player_y: usize,
    pub energy: f32,
    pub robots: Vec<Robot>,
    pub level: i32,
    pub destroyed_count: i32,
}

impl Game {
    pub fn new() -> Self {
        let now = time::SystemTime::now();
        let timestamp = now
            .duration_since(time::UNIX_EPOCH)
            .expect("SystemTime before UNIX EPOCH!")
            .as_secs();
        // let rng = StdRng::seed_from_u64(timestamp);
        println!("random seed = {}", timestamp);
        let rng = StdRng::seed_from_u64(1706226338);

        let mut game = Game {
            rng: Some(rng),
            command_log: Some(File::create("command.log").unwrap()),
            frame: -1,
            is_over: false,
            is_clear: false,
            is_debug: false,
            requested_sounds: Vec::new(),
            commands: Vec::new(),
            replay_loaded: false,
            field: [[EMPTY; FIELD_W]; FIELD_H],
            player_x: 0,
            player_y: 0,
            energy: ENERGY_MAX,
            robots: Vec::new(),
            level: 0,
            destroyed_count: 0,
        };

        game.next_level();

        game.load_replay("replay.dat");

        game
    }

    pub fn toggle_debug(&mut self) {
        self.is_debug = !self.is_debug;
        println!("is_debug: {}", self.is_debug);
    }

    pub fn load_replay(&mut self, filename: &str) {
        if let Some(content) = std::fs::read_to_string(filename).ok() {
            let mut commands = Vec::new();
            for (_, line) in content.lines().enumerate() {
                let command = Command::from_str(line);
                commands.push(command);
            }
            self.replay_loaded = true;
            self.commands = commands;
        }
    }

    pub fn write_command_log(&mut self, command: Command) {
        self.command_log
            .as_ref()
            .unwrap()
            .write_all(format!("{:?}\n", command).as_bytes())
            .ok();
        self.command_log.as_ref().unwrap().flush().ok();
    }

    pub fn next_level(&mut self) {
        self.level += 1;
        self.player_x = FIELD_W / 2;
        self.player_y = FIELD_H / 2;
        self.is_clear = false;
        self.field = [[EMPTY; FIELD_W]; FIELD_H];
        self.robots = Vec::new();
        self.spawn_robots();
    }

    pub fn spawn_robots(&mut self) {
        let robot_count = ROBOT_COUNT + self.level as usize * ROBOT_COUNT_PER_LEVEL;
        while self.robots.len() < robot_count {
            let x = self.rng.as_mut().unwrap().gen_range(0..FIELD_W);
            let y = self.rng.as_mut().unwrap().gen_range(0..FIELD_H);
            let mut should_add = true;
            for robot in &self.robots {
                if robot.x.abs_diff(x) <= 1 && robot.y.abs_diff(y) <= 1 {
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

        if self.replay_loaded {
            if self.commands.len() > self.frame as usize {
                command = self.commands[self.frame as usize];
            }
        } else {
            self.write_command_log(command);
        }

        if self.is_over {
            return;
        }

        if self.is_clear {
            if command == Command::NextLevel {
                self.next_level();
            }
            return;
        }

        if self.energy < ENERGY_MAX {
            self.energy += ENERGY_INCREASE_SPEED;
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
            if self.field[y as usize][x as usize] == JUNK {
                self.requested_sounds.push("ng.wav");
                return;
            }
            self.player_x = x as usize;
            self.player_y = y as usize;
        }
    }

    pub fn teleport(&mut self) {
        if self.energy >= TELEPORT_ENERGY {
            self.energy -= TELEPORT_ENERGY;
            let x = self.rng.as_mut().unwrap().gen_range(0..FIELD_W);
            let y = self.rng.as_mut().unwrap().gen_range(0..FIELD_H);
            self.player_x = x;
            self.player_y = y;
            self.requested_sounds.push("shoot.wav");
        } else {
            self.requested_sounds.push("ng.wav");
        }
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
                if self.field[self.robots[i].y][self.robots[i].x] == JUNK {
                    self.robots[i].exist = false;
                    self.requested_sounds.push("hit.wav");
                }
                for j in (i + 1)..self.robots.len() {
                    if self.robots[i].x == self.robots[j].x && self.robots[i].y == self.robots[j].y
                    {
                        self.field[self.robots[i].y][self.robots[i].x] = JUNK;
                        self.robots[i].exist = false;
                        self.robots[j].exist = false;
                        self.requested_sounds.push("hit.wav");
                    }
                }
            }
        }
        self.destroyed_count += self
            .robots
            .iter()
            .filter(|x| !x.exist)
            .collect::<Vec<_>>()
            .len() as i32;
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
        if self.robots.len() == 0 {
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
