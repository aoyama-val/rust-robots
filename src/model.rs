use rand::prelude::*;
use std::{fs::File, io::Write, time};

pub const FPS: i32 = 30;
pub const FIELD_W: usize = 80;
pub const FIELD_H: usize = 60;
pub const CELL_SIZE: i32 = 10;
pub const EMPTY: i32 = 0;
pub const JUNK: i32 = 1;
pub const ROBOT_TYPE_MIN: i32 = 0;
pub const ROBOT_RED: i32 = 0;
pub const ROBOT_BLUE: i32 = 1;
pub const ROBOT_TYPE_MAX: i32 = 1;
pub const ROBOT_COUNT_MIN: usize = 20;
pub const ROBOT_COUNT_MAX: usize = 30;
pub const ENERGY_MAX: f32 = 100.0;
pub const ENERGY_INCREASE_SPEED: f32 = 0.1;
pub const TELEPORT_ENERGY: f32 = 25.0;
pub const PLAYER_WAIT: i32 = 4;
pub const ROBOTS_WAIT: i32 = 30;

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

#[derive(Debug, Eq, PartialEq)]
pub enum RobotType {
    Red,
    Blue,
}

impl RobotType {
    pub fn from_i32(n: i32) -> Self {
        match n {
            0 => Self::Red,
            1 => Self::Blue,
            _ => panic!(),
        }
    }

    pub fn min() -> i32 {
        0
    }

    pub fn max() -> i32 {
        1
    }
}

#[derive(Debug)]
pub struct Robot {
    pub x: usize,
    pub y: usize,
    pub typ: RobotType,
}

#[derive(Debug)]
pub struct Game {
    pub rng: Option<StdRng>,
    pub is_over: bool,
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
    pub player_wait: i32,
    pub robots_wait: i32,
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
            is_debug: false,
            requested_sounds: Vec::new(),
            commands: Vec::new(),
            replay_loaded: false,
            field: [[EMPTY; FIELD_W]; FIELD_H],
            player_x: FIELD_W / 2,
            player_y: FIELD_H / 2,
            energy: ENERGY_MAX,
            robots: Vec::new(),
            level: 0,
            destroyed_count: 0,
            player_wait: 0,
            robots_wait: ROBOTS_WAIT,
        };

        game.spawn_robots();

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

    pub fn spawn_robots(&mut self) {
        let robot_count = self
            .rng
            .as_mut()
            .unwrap()
            .gen_range(ROBOT_COUNT_MIN..=ROBOT_COUNT_MAX);
        while self.robots.len() < robot_count {
            let x = self.rng.as_mut().unwrap().gen_range(0..FIELD_W);
            let y = self.rng.as_mut().unwrap().gen_range(0..FIELD_H);
            let mut should_add = true;
            for robot in &self.robots {
                if robot.x == x && robot.y == y {
                    should_add = false;
                    break;
                }
            }
            if should_add {
                self.robots.push(Robot {
                    x,
                    y,
                    typ: RobotType::from_i32(
                        self.rng
                            .as_mut()
                            .unwrap()
                            .gen_range(RobotType::min()..=RobotType::max()),
                    ),
                })
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

        if self.energy < ENERGY_MAX {
            self.energy += ENERGY_INCREASE_SPEED;
        }

        wait!(self.player_wait, {
            match command {
                Command::None => {}
                Command::Left => self.move_player(Direction::Left),
                Command::Right => self.move_player(Direction::Right),
                Command::Down => self.move_player(Direction::Down),
                Command::Up => self.move_player(Direction::Up),
                Command::UpLeft => self.move_player(Direction::UpLeft),
                Command::UpRight => self.move_player(Direction::UpRight),
                Command::DownLeft => self.move_player(Direction::DownLeft),
                Command::DownRight => self.move_player(Direction::DownRight),
                Command::Teleport => self.teleport(),
            }
            self.player_wait = PLAYER_WAIT;
        });

        wait!(self.robots_wait, {
            self.move_robots();
            self.robots_wait = ROBOTS_WAIT;
        });
    }

    pub fn move_player(&mut self, direction: Direction) {
        let v = direction.to_vec2();
        let x = self.player_x as i32 + v.x;
        let y = self.player_y as i32 + v.y;
        if 0 <= x && x < FIELD_W as i32 && 0 <= y && y < FIELD_H as i32 {
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
