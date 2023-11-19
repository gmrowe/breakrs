use minifb::{Key, Window, WindowOptions};
use rusttype::{point, Font, Scale};

const WIDTH: usize = 400;
const HEIGHT: usize = 400;

const MAGENTA: u32 = 0xFF00FF;
const CYAN: u32 = 0x00FFFF;

const BG_COLOR: u32 = CYAN;
const BALL_COLOR: u32 = MAGENTA;
const BALL_DIAMETER: usize = 8;

const DEBUG_STATS: bool = true;
const DEBUG_TEXT_SIZE: f32 = 12.0;

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

fn compute_text_data(font: &Font, text_height: f32, text: &str) -> (Vec<u32>, usize) {
    let height = text_height.ceil() as usize;
    let scale = Scale::uniform(text_height);
    let v_metrics = font.v_metrics(scale);
    let offset = point(0.0, v_metrics.ascent);

    let glyphs = font.layout(text, scale, offset).collect::<Vec<_>>();

    let width = glyphs
        .iter()
        .rev()
        .map(|g| g.position().x + g.unpositioned().h_metrics().advance_width)
        .next()
        .unwrap_or(0.0)
        .ceil() as usize;

    let mut text_data = vec![0xFFFFFF_u32; width * height];

    for g in glyphs {
        if let Some(bb) = g.pixel_bounding_box() {
            g.draw(|x, y, v| {
                // v should be in the range 0.0 to 1.0
                let grey = (255.0 * (1.0 - v)) as u32;
                let c = grey << 16 | grey << 8 | grey;

                let x = x as i32 + bb.min.x;
                let y = y as i32 + bb.min.y;
                // There's still a possibility that the glyph clips the boundaries of the bitmap
                if x >= 0 && x < width as i32 && y >= 0 && y < height as i32 {
                    let x = x as usize;
                    let y = y as usize;
                    text_data[x + y * width] = c;
                }
            })
        }
    }
    (text_data, width)
}

fn compute_multiline_text_data(font: &Font, text_height: f32, text: &[&str]) -> (Vec<u32>, usize) {
    let lines = text
        .iter()
        .map(|s| compute_text_data(font, text_height, s))
        .collect::<Vec<_>>();
    let total_len = lines.iter().map(|(data, _)| data.len()).sum();

    let max_stride = lines.iter().map(|&(_, stride)| stride).max().unwrap_or(0);
    let mut multi_line = Vec::with_capacity(total_len);
    for (line, len) in lines.into_iter() {
        let height = line.len() / len;
        if len < max_stride {
            let mut new_line = Vec::new();
            let extension = vec![0xFFFFFF_u32; max_stride - len];
            for y in 0..height {
                new_line.extend_from_slice(&line[len * y..len * y + len]);
                new_line.extend(&extension)
            }
            multi_line.append(&mut new_line);
        } else {
            multi_line.extend_from_slice(&line)
        }
    }
    (multi_line, max_stride)
}

fn render_text(
    canvas: &mut [u32],
    canvas_stride: usize,
    text_data: &[u32],
    text_data_stride: usize,
    pos: (usize, usize),
) {
    let (offset_x, offset_y) = pos;
    let pixel_height = text_data.len() / text_data_stride;
    for y in 0..pixel_height {
        let scanline_start = y * text_data_stride;
        let canvas_start = offset_x + (y + offset_y) * canvas_stride;
        canvas[canvas_start..canvas_start + text_data_stride]
            .copy_from_slice(&text_data[scanline_start..scanline_start + text_data_stride]);
    }
}

fn render_debug_stats(
    canvas: &mut [u32],
    font: &Font,
    text_height: f32,
    ball_pos: (f32, f32),
    ball_vel: (f32, f32),
) {
    let (pos_x, pos_y) = ball_pos;
    let (vel_x, vel_y) = ball_vel;
    let position = format!("pos: ({pos_x:+.3}, {pos_y:+.3})");
    let velocity = format!("vel: ({vel_x:+.3}, {vel_y:+.3})");
    let (text_data, stride) =
        compute_multiline_text_data(font, text_height, &[&position, &velocity]);
    //let data_height = text_data.len() / stride;
    render_text(canvas, WIDTH, &text_data, stride, (0, 0));
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

    let font_path = "fonts/RobotoMono/RobotoMono-VariableFont_wght.ttf";
    let font = {
        let font_path = std::env::current_dir().unwrap().join(font_path);
        let data = std::fs::read(&font_path).unwrap();
        Font::try_from_vec(data).unwrap_or_else(|| {
            panic!("error constructing a Font from data at {:?}", font_path);
        })
    };

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

        if DEBUG_STATS {
            render_debug_stats(&mut buffer, &font, DEBUG_TEXT_SIZE, ball_pos, ball_vel);
        }

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
