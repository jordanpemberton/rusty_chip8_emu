use chip8_core::*;

use std::{env, fs, io, io::Read, path::PathBuf};
use sdl2::{event::Event, EventPump, keyboard::Keycode, pixels::Color, rect::Rect, render::Canvas, render::WindowCanvas, Sdl, video::Window};

const SCALE: u32 = 15;
const WINDOW_WIDTH: u32 = (SCREEN_WIDTH as u32) * SCALE;
const WINDOW_HEIGHT: u32 = (SCREEN_HEIGHT as u32) * SCALE;
const TICKS_PER_FRAME: usize = 8;
const DEFAULT_GAMES_FOLDER_PATH: &str = "/home/jordan/RustProjs/chip8/tests/roms/";

fn draw_screen(emulator: &mut Emulator, canvas: &mut Canvas<Window>) {
    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();

    let screen_buf = emulator.get_display();

    canvas.set_draw_color(Color::RGB(255, 255, 255));

    for (i, pixel) in screen_buf.iter().enumerate() {
        if *pixel {
            let x = (i % SCREEN_WIDTH) as u32;
            let y = (i / SCREEN_WIDTH) as u32;
            let rect = Rect::new((x * SCALE) as i32, (y * SCALE) as i32, SCALE, SCALE);
            canvas.fill_rect(rect).unwrap();
        }
    }
    canvas.present();
}

fn translate_key_input(key: Keycode) -> Option<usize> {
    match key {
        Keycode::Num1 => Some(0x1),
        Keycode::Num2 => Some(0x2),
        Keycode::Num3 => Some(0x3),
        Keycode::Num4 => Some(0xC),

        Keycode::Q => Some(0x4),
        Keycode::W => Some(0x5),
        Keycode::E => Some(0x6),
        Keycode::R => Some(0xD),

        Keycode::A => Some(0x7),
        Keycode::S => Some(0x8),
        Keycode::D => Some(0x9),
        Keycode::F => Some(0xE),

        Keycode::Z => Some(0xA),
        Keycode::X => Some(0x0),
        Keycode::C => Some(0xB),
        Keycode::V => Some(0xF),

        _ => None
    }
}

fn setup_canvas(sdl_context: &Sdl) -> WindowCanvas {
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("Chip-8 Emulator", WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        .opengl()
        .build()
        .unwrap();

    let mut canvas = window
        .into_canvas()
        .present_vsync()
        .build()
        .unwrap();

    canvas.clear();
    canvas.present();
    canvas
}

// Uses index 0 to hold parent dir.
fn get_folder_contents(folder_path: &str) -> Vec<PathBuf> {
    let read_contents = fs::read_dir(folder_path).unwrap();
    let dir_entries: Vec<fs::DirEntry> = read_contents
        .map(|result| result.unwrap())
        .collect();
    let mut path_bufs: Vec<PathBuf> = dir_entries.iter()
        .map(|entry| entry.path())
        .collect();

    path_bufs.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

    let curr_path_buf = path_bufs[0].parent().unwrap().to_path_buf();
    let parent_path_buf = curr_path_buf.parent().unwrap().to_path_buf();
    path_bufs.insert(0, parent_path_buf);

    path_bufs
}

fn display_folder_contents(contents: &[PathBuf]) {
    println!();

    for (i, path) in contents.iter().enumerate() {
        let filename = path.file_name().unwrap();
        println!("{}: {}{}{}",
            i,
            if i == 0 { "↑ " } else { "" },
            filename.to_str().unwrap(),
            if path.is_dir() { "/" } else { "" });
    }
}

fn get_user_input(max_allowed_number: usize) -> usize {
    let mut buffer = String::new();
    let mut x: i32 = -1;

    while x < 0 || x > max_allowed_number as i32 {
        buffer.clear();
        io::stdin().read_line(&mut buffer).unwrap();
        x = buffer.trim().parse::<i32>().expect("Parsed input was not an integer.");
    }

    x as usize
}

fn select_game(games_folder_path: &str) -> String {
    let mut game_full_path: String = String::new();

    let mut files: Vec<PathBuf>;
    let mut selection_number: usize;
    let mut curr_path: &str = games_folder_path;
    let mut sub_path: PathBuf;

    while game_full_path.is_empty() {
        files = get_folder_contents(curr_path);
        display_folder_contents(&files);
        selection_number = get_user_input(files.len());
        sub_path = files[selection_number].clone();
        curr_path = sub_path.to_str().unwrap();
        if !sub_path.is_dir() {
            game_full_path.push_str(curr_path);
        }
    }

    game_full_path
}

fn load_game(game_path: &str) -> Emulator {
    let mut chip8 = Emulator::new();

    let mut msg = String::from("Unable to open file. Provided path was ");
    msg.push_str(game_path);

    let mut rom = fs::File::open(game_path).expect(&*msg);
    let mut buffer = Vec::new();

    rom.read_to_end(&mut buffer).unwrap();
    chip8.load_data(&buffer);
    chip8
}

fn main_loop(chip8: &mut Emulator, event_pump: &mut EventPump, canvas: &mut WindowCanvas) {
    'gameloop: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..}
                    | Event::KeyDown{keycode: Some(Keycode::Escape), ..} => {
                    break 'gameloop;
                },
                Event::KeyDown{keycode: Some(key), ..} => {
                    if let Some(chip8_input) = translate_key_input(key) {
                        chip8.keypress(chip8_input, true);
                    }
                },
                Event::KeyUp{keycode: Some(key), ..} => {
                    if let Some(chip8_input) = translate_key_input(key) {
                        chip8.keypress(chip8_input, false);
                    }
                }
                _ => ()
            }
        }

        for _ in 0..TICKS_PER_FRAME {
            chip8.tick();
        }
        chip8.tick_timers();

        draw_screen(chip8, canvas);
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut games_folder_path: String = if args.len() > 1 { args[1].clone() }
        else { DEFAULT_GAMES_FOLDER_PATH.to_string() };
    if !games_folder_path.ends_with('/') {
        games_folder_path.push('/');
    }

    let game_full_path: String = select_game(games_folder_path.as_str());

    if !game_full_path.is_empty() {
        let sdl_context: Sdl = sdl2::init().unwrap();

        let mut canvas: WindowCanvas = setup_canvas(&sdl_context);
        let mut event_pump: EventPump = sdl_context.event_pump().unwrap();

        let mut game: Emulator = load_game(game_full_path.as_str());

        main_loop(&mut game, &mut event_pump, &mut canvas);
    }
}
