use minifb::{Key, Window, WindowOptions};
use rusttype::{point, Font, Scale};

const WIDTH: usize = 600;
const HEIGHT: usize = 600;

const MAGENTA: u32 = 0xFF00FF;
const CYAN: u32 = 0x00FFFF;
const YELLOW: u32 = 0xFFFF00;

type Res<T> = Result<T, ()>;

fn dot_product(x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    x1 * x2 + y1 * y2
}

fn magnitude(x: f32, y: f32) -> f32 {
    (x * x + y * y).sqrt()
}

fn normalize(x: f32, y: f32) -> (f32, f32) {
    let sum_of_sqaures = x * x + y * y;
    let magnitude = sum_of_sqaures.sqrt();
    (x / magnitude, y / magnitude)
}

fn reflect(x: f32, y: f32, x_norm: f32, y_norm: f32) -> (f32, f32) {
    let (xn, yn) = normalize(x_norm, y_norm);
    let dot = dot_product(x, y, xn, yn);
    let x_reflect = x_norm * 2.0 * dot;
    let y_reflect = y_norm * 2.0 * dot;
    (x - x_reflect, y - y_reflect)
}

fn to_screen_coords(
    world_x: f32,
    world_y: f32,
    screen_width: usize,
    screen_height: usize,
) -> (usize, usize) {
    let x = (screen_width as f32 * (1.0 + world_x) / 2.0) as usize;
    let y = screen_height - (screen_height as f32 * (1.0 + world_y) / 2.0) as usize;
    (x, y)
}

fn draw_circle(canvas: &mut Canvas, x: usize, y: usize, diameter: usize, color: u32) {
    let radius = diameter / 2;
    let center_x = x + radius;
    let center_y = y + radius;
    for row in y..y + diameter {
        for col in x..x + diameter {
            let delta_x = center_x.abs_diff(col);
            let delta_y = center_y.abs_diff(row);
            if delta_x * delta_x + delta_y * delta_y <= radius * radius {
                canvas.buffer[row * canvas.stride + col] = color;
            }
        }
    }
}

fn draw_rect(canvas: &mut Canvas, x: usize, y: usize, width: usize, height: usize, color: u32) {
    for row in y..y + height {
        let start = x + canvas.stride * row;
        canvas.buffer[start..start + width].fill(color);
    }
}

fn compute_text_data(font: &Font, text_height: f32, text: &str) -> Canvas {
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
    Canvas {
        buffer: text_data,
        stride: width,
    }
}

fn compute_multiline_text_data(font: &Font, text_height: f32, text: &[&str]) -> Canvas {
    let lines = text
        .iter()
        .map(|s| compute_text_data(font, text_height, s))
        .collect::<Vec<_>>();
    let max_stride = lines.iter().map(|canvas| canvas.stride).max().unwrap_or(0);
    let mut multi_line = Vec::new();
    for canvas in lines.into_iter() {
        let height = canvas.buffer.len() / canvas.stride;
        let mut new_line = Vec::new();
        let extension = vec![0xFFFFFF_u32; max_stride - canvas.stride];
        for y in 0..height {
            new_line.extend_from_slice(
                &canvas.buffer[canvas.stride * y..canvas.stride * y + canvas.stride],
            );
            new_line.extend(&extension)
        }
        multi_line.append(&mut new_line);
    }
    Canvas {
        buffer: multi_line,
        stride: max_stride,
    }
}

fn draw_subcanvas(canvas: &mut Canvas, subcanvas: &Canvas, pos: (usize, usize)) {
    let (offset_x, offset_y) = pos;
    let pixel_height = subcanvas.buffer.len() / subcanvas.stride;
    for y in 0..pixel_height {
        let scanline_start = y * subcanvas.stride;
        let canvas_start = offset_x + (y + offset_y) * canvas.stride;
        canvas.buffer[canvas_start..canvas_start + subcanvas.stride]
            .copy_from_slice(&subcanvas.buffer[scanline_start..scanline_start + subcanvas.stride]);
    }
}

fn game_loop(window: &mut Window, game_state: &mut GameState, canvas: &mut Canvas) -> Res<()> {
    while window.is_open() && !window.is_key_down(Key::Escape) {
        game_state.tick();
        game_state.draw_all(canvas);

        window
            .update_with_buffer(&canvas.buffer, WIDTH, HEIGHT)
            .map_err(|err| {
                eprintln!("ERROR! Failed to update window: {err}");
            })?;

        window.get_keys().iter().for_each(|key| match key {
            Key::LeftShift | Key::RightShift => {
                if window.is_key_down(Key::Equal) {
                    game_state.update_ball_speed(1.05);
                }
            }

            Key::Minus => {
                game_state.update_ball_speed(0.95);
            }

            Key::A => {
                game_state.paddle_vel_x = -game_state.paddle_movement_speed;
            }

            Key::D => {
                game_state.paddle_vel_x = game_state.paddle_movement_speed;
            }

            _ => (),
        });

        window.get_keys_released().iter().for_each(|key| match key {
            Key::A | Key::D => {
                game_state.paddle_vel_x = 0.0;
            }
            _ => (),
        });
    }
    Ok(())
}

struct Canvas {
    buffer: Vec<u32>,
    stride: usize,
}

impl Canvas {
    fn width(&self) -> usize {
        self.stride
    }

    fn height(&self) -> usize {
        self.buffer.len() / self.stride
    }
}

struct Bricks {
    x_positions: Vec<f32>,
    y_positions: Vec<f32>,
    colors: Vec<u32>,
    width: f32,
    height: f32,
}

impl Bricks {
    fn new() -> Self {
        let mut x_positions = Vec::new();
        let mut y_positions = Vec::new();
        let mut colors = Vec::new();
        let brick_count = 13;
        let brick_coverage = 0.90 * 2.0;
        let width = 0.1385;
        let gap_count = 14;
        let gap_width = (2.0 - brick_coverage) / gap_count as f32;
        let height = width / 3.0;
        let brick_color = 0x00FF00_u32;
        for b in 0..brick_count {
            let brick_x_pos = -1.0 + ((b + 1) as f32 * gap_width) + (b as f32 * width);
            let brick_y_pos = 0.25;
            x_positions.push(brick_x_pos);
            y_positions.push(brick_y_pos);
            colors.push(brick_color);
        }
        Bricks {
            x_positions,
            y_positions,
            colors,
            width,
            height,
        }
    }
}

struct GameState {
    debug_stats: bool,
    debug_stats_height: f32,
    font: Option<Font<'static>>,
    ball_pos_x: f32,
    ball_pos_y: f32,
    ball_vel_x: f32,
    ball_vel_y: f32,
    ball_diameter: f32,
    ball_color: u32,
    background_color: u32,
    paddle_pos_x: f32,
    paddle_pos_y: f32,
    paddle_width: f32,
    paddle_height: f32,
    paddle_vel_x: f32,
    paddle_movement_speed: f32,
    paddle_color: u32,
    bricks: Bricks,
}

impl GameState {
    fn paddle_collision(&self) -> Option<f32> {
        let dx = self.ball_pos_x + self.ball_vel_x;
        let dy = self.ball_pos_y + self.ball_vel_y;

        if self.ball_vel_y < 0.0
            && dx + self.ball_diameter >= self.paddle_pos_x
            && dx < self.paddle_pos_x + self.paddle_width
            && dy - self.ball_diameter <= self.paddle_pos_y
            && dy >= self.paddle_pos_y - self.paddle_height
        {
            let extreme_left = self.paddle_pos_x - self.ball_diameter;
            let extreme_right = self.paddle_pos_x + self.paddle_width;
            let hit_location = (dx - extreme_left) / (extreme_right - extreme_left);
            Some(hit_location)
        } else {
            None
        }
    }

    fn update_ball_pos(&mut self) {
        let max_x = 1.0 - self.ball_diameter;
        let min_y = -1.0 + self.ball_diameter;

        let dx = self.ball_pos_x + self.ball_vel_x;
        let dy = self.ball_pos_y + self.ball_vel_y;

        // Check for paddle collision
        let sqrt_3 = 3.0_f32.sqrt();
        if let Some(location) = self.paddle_collision() {
            const PADDLE_DIV: f32 = 1.0 / 3.0;
            let (rx, ry) = if location < PADDLE_DIV {
                (-1.0, sqrt_3)
            } else if location < PADDLE_DIV * 2.0 {
                (0.0, 1.0)
            } else {
                (1.0, sqrt_3)
            };
            let original_magnitude = magnitude(self.ball_vel_x, self.ball_vel_y);
            let (vx, vy) = reflect(self.ball_vel_x, self.ball_vel_y, rx, ry);
            let (nvx, nvy) = normalize(vx, vy);
            self.ball_vel_x = nvx * original_magnitude;
            self.ball_vel_y = nvy * original_magnitude;
        }

        // Check for side walls collision
        if dx <= -1.0 || dx >= max_x {
            self.ball_vel_x = -self.ball_vel_x;
        }

        // Check for top and bottom wall collision
        if dy <= min_y || dy >= 1.0 {
            self.ball_vel_y = -self.ball_vel_y;
        }

        self.ball_pos_x = if dx > max_x {
            max_x - (dx - max_x)
        } else if dx < -1.0 {
            -1.0 + (-1.0 - dx)
        } else {
            dx
        };

        self.ball_pos_y = if dy > 1.0 {
            1.0 - (dy - 1.0)
        } else if dy < min_y {
            min_y + (min_y - dy)
        } else {
            dy
        };
    }

    fn update_paddle_pos(&mut self) {
        let max_x = 1.0 - self.paddle_width;
        self.paddle_pos_x = (self.paddle_pos_x + self.paddle_vel_x).clamp(-1.0, max_x);
    }

    fn tick(&mut self) {
        self.update_ball_pos();
        self.update_paddle_pos();
    }

    fn update_ball_speed(&mut self, factor: f32) {
        self.ball_vel_x *= factor;
        self.ball_vel_y *= factor;
    }

    fn draw_ball(&self, canvas: &mut Canvas) {
        let (x, y) = to_screen_coords(
            self.ball_pos_x,
            self.ball_pos_y,
            canvas.width(),
            canvas.height(),
        );
        let screen_diameter = (self.ball_diameter * canvas.stride as f32 / 2.0) as usize;
        draw_circle(canvas, x, y, screen_diameter, self.ball_color);
    }

    fn draw_paddle(&self, canvas: &mut Canvas) {
        let (x, y) = to_screen_coords(
            self.paddle_pos_x,
            self.paddle_pos_y,
            canvas.width(),
            canvas.height(),
        );
        let screen_height = canvas.buffer.len() / canvas.stride;
        let width = (self.paddle_width / 2.0 * canvas.stride as f32) as usize;
        let height = (self.paddle_height / 2.0 * screen_height as f32) as usize;
        draw_rect(canvas, x, y, width, height, self.paddle_color);
    }

    fn draw_debug_stats(&self, canvas: &mut Canvas) {
        let ball_position = format!(
            "{pos:<12} ({pos_x:+.3}, {pos_y:+.3})",
            pos = "pos:",
            pos_x = self.ball_pos_x,
            pos_y = self.ball_pos_y
        );
        let ball_velocity = format!(
            "{vel:<12} ({vel_x:+.3}, {vel_y:+.3})",
            vel = "vel:",
            vel_x = self.ball_vel_x,
            vel_y = self.ball_vel_y
        );
        let paddle_pos = format!(
            "{pos:<12} ({pos_x:+.3}, {pos_y:+.3})",
            pos = "paddle_pos:",
            pos_x = self.paddle_pos_x,
            pos_y = self.paddle_pos_y
        );
        let text_canvas = compute_multiline_text_data(
            self.font
                .as_ref()
                .expect("Method is only called if font.is_some()"),
            self.debug_stats_height,
            &[&ball_position, &ball_velocity, &paddle_pos],
        );
        draw_subcanvas(canvas, &text_canvas, (0, 0));
    }

    fn draw_all(&self, canvas: &mut Canvas) {
        canvas.buffer.fill(self.background_color);
        self.draw_ball(canvas);
        self.draw_paddle(canvas);
        if self.debug_stats && self.font.is_some() {
            self.draw_debug_stats(canvas);
        }

        let width = (self.bricks.width / 2.0 * canvas.width() as f32).ceil() as usize;
        let height = (self.bricks.height / 2.0 * canvas.height() as f32).ceil() as usize;
        for ((brick_x, brick_y), color) in self
            .bricks
            .x_positions
            .iter()
            .zip(self.bricks.y_positions.iter())
            .zip(self.bricks.colors.iter())
        {
            let (x, y) = to_screen_coords(*brick_x, *brick_y, canvas.width(), canvas.height());
            draw_rect(canvas, x, y, width, height, *color);
        }
    }
}

impl Default for GameState {
    fn default() -> Self {
        GameState {
            font: None,
            debug_stats: true,
            debug_stats_height: 16.0,
            ball_pos_x: 0.0,
            ball_pos_y: 0.0,
            ball_vel_x: 0.0039,
            ball_vel_y: 0.0024,
            ball_diameter: 0.032,
            ball_color: MAGENTA,
            background_color: CYAN,
            paddle_pos_x: -0.04,
            paddle_pos_y: -0.8,
            paddle_width: 0.2,
            paddle_height: 0.02,
            paddle_vel_x: 0.0,
            paddle_movement_speed: 0.022,
            paddle_color: YELLOW,
            bricks: Bricks::new(),
        }
    }
}

pub fn main() -> Res<()> {
    let mut canvas = Canvas {
        buffer: vec![0; WIDTH * HEIGHT],
        stride: WIDTH,
    };

    let font_path = "fonts/RobotoMono/RobotoMono-VariableFont_wght.ttf";
    let font = {
        let font_path = std::env::current_dir().unwrap().join(font_path);
        let data = std::fs::read(&font_path).unwrap();
        Font::try_from_vec(data).unwrap_or_else(|| {
            panic!("error constructing a Font from data at {:?}", font_path);
        })
    };

    let mut game_state = GameState {
        font: Some(font),
        debug_stats: true,
        ..GameState::default()
    };

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
    game_loop(&mut window, &mut game_state, &mut canvas)?;
    Ok(())
}
