use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mixer;
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::{BlendMode, Canvas, Texture, TextureCreator};
use sdl2::ttf::Sdl2TtfContext;
use sdl2::video::{Window, WindowContext};
use std::collections::HashMap;
use std::fs;
use std::time::{Duration, SystemTime};
mod model;
use crate::model::*;

pub const WINDOW_TITLE: &str = "rust-robots";
pub const SCREEN_WIDTH: i32 = FIELD_W * CELL_W;
pub const SCREEN_HEIGHT: i32 = FIELD_H * CELL_H + INFO_HEIGHT;
pub const INFO_HEIGHT: i32 = 28;
pub const SOUND_WAIT: i32 = 4;

struct Image<'a> {
    texture: Texture<'a>,
    #[allow(dead_code)]
    w: u32,
    h: u32,
}

impl<'a> Image<'a> {
    fn new(texture: Texture<'a>) -> Self {
        let q = texture.query();
        Image {
            texture,
            w: q.width,
            h: q.height,
        }
    }
}

struct Resources<'a> {
    images: HashMap<String, Image<'a>>,
    chunks: HashMap<String, sdl2::mixer::Chunk>,
    fonts: HashMap<String, sdl2::ttf::Font<'a, 'a>>,
}

pub fn main() -> Result<(), String> {
    let sdl_context = sdl2::init()?;

    let video_subsystem = sdl_context.video()?;
    let window = video_subsystem
        .window(WINDOW_TITLE, SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32)
        .position_centered()
        .opengl()
        .build()
        .map_err(|e| e.to_string())?;

    sdl_context.mouse().show_cursor(false);

    init_mixer();
    let mut sound_wait: i32 = 0;

    let ttf_context = sdl2::ttf::init().map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
    canvas.set_blend_mode(BlendMode::Blend);

    let texture_creator = canvas.texture_creator();
    let mut resources = load_resources(&texture_creator, &mut canvas, &ttf_context);

    let mut event_pump = sdl_context.event_pump()?;

    let mut game = Game::new();

    println!("h      : Move left");
    println!("j      : Move down");
    println!("k      : Move up");
    println!("l      : Move right");
    println!("y      : Move up and left");
    println!("u      : Move up and right");
    println!("b      : Move down and left");
    println!("n      : Move down and right");
    println!("t      : Teleport to a random location");
    println!(".      : Do nothing for one turn");
    println!("Enter  : Restart");

    'running: loop {
        let started = SystemTime::now();

        let mut command = Command::None;

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'running,
                Event::KeyDown {
                    keycode: Some(code),
                    ..
                } => {
                    match code {
                        Keycode::Escape => break 'running,
                        Keycode::Return => {
                            if !game.is_clear {
                                game = Game::new();
                            } else {
                                command = Command::NextLevel;
                            }
                        }
                        Keycode::H => command = Command::Left,
                        Keycode::L => command = Command::Right,
                        Keycode::K => command = Command::Up,
                        Keycode::J => command = Command::Down,
                        Keycode::Y => command = Command::UpLeft,
                        Keycode::U => command = Command::UpRight,
                        Keycode::B => command = Command::DownLeft,
                        Keycode::N => command = Command::DownRight,
                        Keycode::T => command = Command::Teleport,
                        Keycode::Period => command = Command::Wait,
                        _ => {
                            game.requested_sounds.push("ng.wav");
                        }
                    };
                }
                _ => {}
            }
        }
        game.update(command);
        render(&mut canvas, &game, &mut resources)?;

        model::wait!(sound_wait, {
            if !game.requested_sounds.is_empty() {
                let sound_key = game.requested_sounds.remove(0);
                let chunk = resources
                    .chunks
                    .get(&sound_key.to_string())
                    .expect("cannot get sound");
                sdl2::mixer::Channel::all()
                    .play(chunk, 0)
                    .expect("cannot play sound");
                sound_wait = SOUND_WAIT;
            }
        });

        let finished = SystemTime::now();
        let elapsed = finished.duration_since(started).unwrap();
        let frame_duration = Duration::new(0, 1_000_000_000u32 / model::FPS as u32);
        if elapsed < frame_duration {
            ::std::thread::sleep(frame_duration - elapsed)
        }
    }

    Ok(())
}

fn init_mixer() {
    let chunk_size = 1_024;
    mixer::open_audio(
        mixer::DEFAULT_FREQUENCY,
        mixer::DEFAULT_FORMAT,
        mixer::DEFAULT_CHANNELS,
        chunk_size,
    )
    .expect("cannot open audio");
    let _mixer_context = mixer::init(mixer::InitFlag::MP3).expect("cannot init mixer");
}

fn load_resources<'a>(
    texture_creator: &'a TextureCreator<WindowContext>,
    #[allow(unused_variables)] canvas: &mut Canvas<Window>,
    ttf_context: &'a Sdl2TtfContext,
) -> Resources<'a> {
    let mut resources = Resources {
        images: HashMap::new(),
        chunks: HashMap::new(),
        fonts: HashMap::new(),
    };

    let entries = fs::read_dir("resources/image").unwrap();
    for entry in entries {
        let path = entry.unwrap().path();
        let path_str = path.to_str().unwrap();
        if path_str.ends_with(".bmp") {
            let temp_surface = sdl2::surface::Surface::load_bmp(&path).unwrap();
            let texture = texture_creator
                .create_texture_from_surface(&temp_surface)
                .unwrap_or_else(|_| panic!("cannot load image: {}", path_str));

            let basename = path.file_name().unwrap().to_str().unwrap();
            let image = Image::new(texture);
            resources.images.insert(basename.to_string(), image);
        }
    }

    let entries = fs::read_dir("./resources/sound").unwrap();
    for entry in entries {
        let path = entry.unwrap().path();
        let path_str = path.to_str().unwrap();
        if path_str.ends_with(".wav") {
            let chunk = mixer::Chunk::from_file(path_str)
                .unwrap_or_else(|_| panic!("cannot load sound: {}", path_str));
            let basename = path.file_name().unwrap().to_str().unwrap();
            resources.chunks.insert(basename.to_string(), chunk);
        }
    }

    load_font(
        &mut resources,
        ttf_context,
        "./resources/font/boxfont2.ttf",
        24,
        "boxfont",
    );

    resources
}

fn load_font<'a>(
    resources: &mut Resources<'a>,
    ttf_context: &'a Sdl2TtfContext,
    path_str: &str,
    point_size: u16,
    key: &str,
) {
    let font = ttf_context
        .load_font(path_str, point_size)
        .unwrap_or_else(|_| panic!("cannot load font: {}", path_str));
    resources.fonts.insert(key.to_string(), font);
}

fn render(
    canvas: &mut Canvas<Window>,
    game: &Game,
    resources: &mut Resources,
) -> Result<(), String> {
    canvas.set_draw_color(Color::RGB(32, 32, 32));
    canvas.clear();

    let font = resources.fonts.get_mut("boxfont").unwrap();

    // render junks
    for junk in &game.junks {
        canvas.set_draw_color(Color::RGB(255, 255, 128));
        canvas.fill_rect(Rect::new(
            junk.pos.x * CELL_W,
            junk.pos.y * CELL_H + INFO_HEIGHT,
            CELL_W as u32,
            CELL_H as u32,
        ))?;
    }

    // render player
    canvas.set_draw_color(Color::RGB(192, 192, 192));
    canvas.fill_rect(Rect::new(
        game.player.pos.x * CELL_W,
        game.player.pos.y * CELL_H + INFO_HEIGHT,
        CELL_W as u32,
        CELL_H as u32,
    ))?;

    // render robots
    for robot in &game.robots {
        let color = Color::RGB(255, 128, 128);
        canvas.set_draw_color(color);
        canvas.fill_rect(Rect::new(
            robot.pos.x * CELL_W,
            robot.pos.y * CELL_H + INFO_HEIGHT,
            CELL_W as u32,
            CELL_H as u32,
        ))?;
    }

    // render laser cannon
    let laser_cannon_image = resources.images.get_mut("laser_cannon.bmp").unwrap();
    canvas
        .copy_ex(
            &laser_cannon_image.texture,
            None,
            Rect::new(
                game.laser_cannon.pos.x * CELL_W,
                game.laser_cannon.pos.y * CELL_H + INFO_HEIGHT,
                CELL_W as u32,
                CELL_H as u32,
            ),
            match game.laser_cannon.direction {
                Direction::Left => -90.0,
                Direction::Right => 90.0,
                Direction::Up => 0.0,
                Direction::Down => 180.0,
                _ => panic!(),
            }, /* SDLのangleは時計回りが正 */
            Point::new(CELL_W / 2, CELL_H / 2),
            false,
            false,
        )
        .unwrap();

    // render laser
    if game.laser_cannon.is_shooting {
        let laser_color = Color::RGB(255, 255, 128);
        canvas.set_draw_color(laser_color);
        if game.laser_cannon.direction == Direction::Left
            || game.laser_cannon.direction == Direction::Right
        {
            canvas.fill_rect(Rect::new(
                game.laser_cannon.begin * CELL_W,
                game.laser_cannon.pos.y * CELL_H + INFO_HEIGHT + 6,
                ((game.laser_cannon.end - game.laser_cannon.begin) * CELL_W) as u32,
                4,
            ))?;
        } else {
            canvas.fill_rect(Rect::new(
                game.laser_cannon.pos.x * CELL_W + 6,
                game.laser_cannon.begin * CELL_H + INFO_HEIGHT,
                4,
                ((game.laser_cannon.end - game.laser_cannon.begin) * CELL_H) as u32,
            ))?;
        }
    }

    // render info
    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.fill_rect(Rect::new(0, 0, SCREEN_WIDTH as u32, INFO_HEIGHT as u32))?;

    let font_color = Color::RGB(224, 224, 224);
    render_font(
        canvas,
        font,
        format!(
            "LEVEL {}   ROBOTS {}/{}",
            game.level,
            game.robots.len(),
            game.initial_robot_count
        )
        .to_string(),
        0,
        0,
        font_color,
        false,
    );

    if game.is_over {
        canvas.set_draw_color(Color::RGBA(255, 0, 0, 128));
        canvas.fill_rect(Rect::new(0, 0, SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32))?;
    }

    if game.is_clear {
        let colors = [
            Color::RGB(255, 128, 128),
            Color::RGB(128, 255, 128),
            Color::RGB(128, 128, 255),
            Color::RGB(255, 255, 128),
            Color::RGB(128, 255, 255),
            Color::RGB(255, 128, 255),
        ];
        let color = colors[game.frame as usize % colors.len()];
        render_font(
            canvas,
            font,
            "YOU WIN!".to_string(),
            SCREEN_WIDTH / 2,
            SCREEN_HEIGHT / 2 - 20,
            color,
            true,
        );
        render_font(
            canvas,
            font,
            "PRESS ENTER TO NEXT LEVEL".to_string(),
            SCREEN_WIDTH / 2,
            SCREEN_HEIGHT / 2 + 20,
            Color::RGB(255, 255, 255),
            true,
        );
    }

    canvas.present();

    Ok(())
}

fn render_font(
    canvas: &mut Canvas<Window>,
    font: &sdl2::ttf::Font,
    text: String,
    x: i32,
    y: i32,
    color: Color,
    center: bool,
) {
    let texture_creator = canvas.texture_creator();

    let surface = font.render(&text).blended(color).unwrap();
    let texture = texture_creator
        .create_texture_from_surface(&surface)
        .unwrap();
    let x: i32 = if center {
        x - texture.query().width as i32 / 2
    } else {
        x
    };
    canvas
        .copy(
            &texture,
            None,
            Rect::new(x, y, texture.query().width, texture.query().height),
        )
        .unwrap();
}
