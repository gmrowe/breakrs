use minifb::{Key, Window, WindowOptions};

const WIDTH: usize = 400;
const HEIGHT: usize = 400;

const MAGENTA: u32 = 0xFF00FF;
const CYAN: u32 = 0x00FFFF;

const BG_COLOR: u32 = CYAN;
const BALL_COLOR: u32 = MAGENTA;
const BALL_WIDTH: usize = 8;
const BALL_HEIGHT: usize = 8;

type Res<T> = Result<T, ()>;

fn to_screen_coords(world_coords: (f32, f32)) -> (usize, usize) {
    let (world_x, world_y) = world_coords;
    let half_width = (WIDTH as f32) / 2.0;
    let half_height = (HEIGHT as f32) / 2.0;
    let x = (half_width + half_width * world_x) as usize;
    let y = (half_height + half_height * world_y) as usize;
    (x, y)
}

fn draw_ball(canvas: &mut [u32], pos: (usize, usize)) {
    let (x, y) = pos;
    canvas.fill(BG_COLOR);

    for row in y..y + BALL_HEIGHT {
        let col0 = row * WIDTH + x;
        canvas[col0..col0 + BALL_WIDTH].fill(BALL_COLOR);
    }
}

pub fn main() -> Res<()> {
    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];

    let mut window = Window::new(
        "BREAKRS - ESC to exit",
        WIDTH,
        HEIGHT,
        WindowOptions::default(),
    )
    .map_err(|err| {
        eprintln!("ERROR! Could not create window: {err}");
    })?;

    // Limit to max ~60 fps update rate
    window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));

    let mut ball_pos: (f32, f32) = (0.0, 0.0);
    let mut ball_vel: (f32, f32) = (0.05, -0.08);
    const MAX_X: f32 = 1.0 - (BALL_WIDTH as f32 / WIDTH as f32 * 2.0);
    const MAX_Y: f32 = 1.0 - (BALL_HEIGHT as f32 / HEIGHT as f32 * 2.0);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let (x, y) = ball_pos;
        let (dx, dy) = ball_vel;

        let tick_x = (x + dx).clamp(-1.0, MAX_X);
        let tick_y = (y + dy).clamp(-1.0, MAX_Y);

        let dir_x = if tick_x > -1.0 && tick_x < MAX_X {
            dx
        } else {
            -dx
        };

        let dir_y = if tick_y > -1.0 && tick_y < MAX_Y {
            dy
        } else {
            -dy
        };

        ball_pos = (tick_x, tick_y);
        ball_vel = (dir_x, dir_y);

        draw_ball(&mut buffer, to_screen_coords(ball_pos));
        // We unwrap here as we want this code to exit if it fails. Real applications may want to handle this in a different way
        window
            .update_with_buffer(&buffer, WIDTH, HEIGHT)
            .map_err(|err| {
                eprintln!("ERROR! Failed to update window: {err}");
            })?;

        window.get_keys().iter().for_each(|key| match key {
            Key::LeftShift | Key::RightShift => {
                if window.is_key_down(Key::Equal) {
                    let (dx, dy) = ball_vel;
                    ball_vel = (dx * 1.05, dy * 1.05);
                }
            }

            _ => (),
        });
    }

    Ok(())
}
