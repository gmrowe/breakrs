use minifb::{Key, Window, WindowOptions};

const WIDTH: usize = 400;
const HEIGHT: usize = 400;

const MAGENTA: u32 = 0xFF00FF;
const CYAN: u32 = 0x00FFFF;

const BG_COLOR: u32 = CYAN;
const BALL_COLOR: u32 = MAGENTA;
const BALL_DIAMETER: usize = 8;

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
    let radius = BALL_DIAMETER / 2;
    let (center_x, center_y) = (x + radius, y + radius);
    canvas.fill(BG_COLOR);

    for row in y..y + BALL_DIAMETER {
        for col in x..x + BALL_DIAMETER {
            let delta_x = center_x.abs_diff(col);
            let delta_y = center_y.abs_diff(row);
            if delta_x * delta_x + delta_y * delta_y < radius * radius {
                let index = row * WIDTH + col;
                canvas[index] = BALL_COLOR;
            }
        }
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
    let mut ball_vel: (f32, f32) = (0.005, -0.002);
    const MAX_X: f32 = 1.0 - (BALL_DIAMETER as f32 / WIDTH as f32 * 2.0);
    const MAX_Y: f32 = 1.0 - (BALL_DIAMETER as f32 / HEIGHT as f32 * 2.0);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let (x, y) = ball_pos;
        let (dx, dy) = ball_vel;

        let tick_x = x + dx;
        let tick_y = y + dy;

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

        let pos_x = if tick_x > MAX_X {
            MAX_X - (tick_x - MAX_X)
        } else if tick_x < -1.0 {
            -1.0 + (-1.0 - tick_x)
        } else {
            tick_x
        };

        let pos_y = if tick_y > MAX_Y {
            MAX_Y - (tick_y - MAX_Y)
        } else if tick_y < -1.0 {
            -1.0 + (-1.0 - tick_y)
        } else {
            tick_y
        };

        ball_pos = (pos_x, pos_y);
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

            Key::Minus => {
                let (dx, dy) = ball_vel;
                ball_vel = (dx * 0.95, dy * 0.95);
            }

            _ => (),
        });
    }

    Ok(())
}
