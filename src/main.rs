//mod myrts;

use {
    pixels::{Pixels, SurfaceTexture},
    std::{
        collections::HashMap,
        ffi::CString,
        io::Write,
        ops::{Add, Mul},
    },
    winit::{
        dpi::{PhysicalPosition, PhysicalSize},
        event::{
            device::{GamepadHandle, HidId, KeyboardId, MouseEvent, MouseId},
            ElementState, Event, Force, KeyboardInput, MouseScrollDelta, Touch, TouchPhase,
            VirtualKeyCode, WindowEvent,
        },
        event_loop::{ControlFlow, EventLoop},
        platform::windows::DeviceExtWindows,
        window::{Window, WindowBuilder},
    },
};

type Color = [u8; 3];

#[derive(Default, Debug, Clone, Copy)]
struct Point {
    pos: PhysicalPosition<f64>,
    pressure: f64,
}

impl Mul<f64> for Point {
    type Output = Point;
    fn mul(self, rhs: f64) -> Self::Output {
        Point {
            pos: PhysicalPosition {
                x: self.pos.x * rhs,
                y: self.pos.y * rhs,
            },
            // ??
            pressure: self.pressure * rhs,
        }
    }
}

impl Add for Point {
    type Output = Point;
    fn add(self, rhs: Self) -> Self::Output {
        Point {
            pos: PhysicalPosition {
                x: self.pos.x + rhs.pos.x,
                y: self.pos.y + rhs.pos.y,
            },
            // ????
            pressure: self.pressure + rhs.pressure,
        }
    }
}

#[derive(Default, Debug)]
struct Stroke {
    points: Vec<Point>,
    color: Color,
    brush_size: f64,
    style: StrokeStyle,
    erased: bool,
}

#[derive(Debug, Clone, Copy, evc_derive::EnumVariantCount)]
#[repr(usize)]
#[allow(dead_code)]
enum StrokeStyle {
    Lines,
    Circles,
    CirclesPressure,
    Points,
}

impl Default for StrokeStyle {
    fn default() -> Self {
        StrokeStyle::Lines
    }
}

#[derive(Debug, Clone, Copy)]
enum KeyState {
    Downstroke,
    Held,
    Released,
}

impl KeyState {
    fn is_down(&self) -> bool {
        use KeyState::*;
        matches!(self, Downstroke | Held)
    }

    fn just_pressed(&self) -> bool {
        use KeyState::*;
        matches!(self, Downstroke)
    }
}

#[derive(Debug, Clone, Copy)]
enum StylusPosition {
    Down,
    Up,
}

#[derive(Debug, Clone, Copy)]
struct StylusState {
    pos: StylusPosition,
    inverted: bool,
}

impl Default for StylusState {
    fn default() -> Self {
        StylusState {
            pos: StylusPosition::Up,
            inverted: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct Stylus {
    state: StylusState,
    pressure: f64,
    pos: PhysicalPosition<f64>,
}

impl Stylus {
    fn down(&self) -> bool {
        matches!(self.state.pos, StylusPosition::Down)
    }

    fn inverted(&self) -> bool {
        self.state.inverted
    }
}

#[derive(Default)]
struct State {
    stylus: Stylus,
    brush_size: f64,
    fill_brush_head: bool,
    strokes: Vec<Stroke>,
    keys: HashMap<VirtualKeyCode, KeyState>,
    style: StrokeStyle,
    use_individual_style: bool,
}

impl State {
    fn init(&mut self) {
        self.brush_size = 1.0;
    }

    fn key(&mut self, key: VirtualKeyCode, element_state: ElementState) {
        let key_state = self.keys.entry(key).or_insert(KeyState::Released);

        let next_key_state = match (*key_state, element_state) {
            (KeyState::Released, ElementState::Pressed) => KeyState::Downstroke,
            (_, ElementState::Released) => KeyState::Released,
            (_, ElementState::Pressed) => KeyState::Held,
        };

        *key_state = next_key_state;
    }

    fn is_down(&self, key: VirtualKeyCode) -> bool {
        self.keys.contains_key(&key) && self.keys[&key].is_down()
    }

    fn just_pressed(&self, key: VirtualKeyCode) -> bool {
        self.keys.contains_key(&key) && self.keys[&key].just_pressed()
    }

    fn shift(&self) -> bool {
        use VirtualKeyCode::{LShift, RShift};
        self.is_down(LShift) || self.is_down(RShift)
    }

    fn control(&self) -> bool {
        use VirtualKeyCode::{LControl, RControl};
        self.is_down(LControl) || self.is_down(RControl)
    }

    fn rotate_style(&mut self) {
        let style_num = self.style as usize;
        let next_num = (style_num + 1) % StrokeStyle::NUM_VARIANTS;
        self.style = unsafe { std::mem::transmute(next_num) };
    }

    fn increase_brush(&mut self) {
        let max_brush = 32.0;
        if self.brush_size + 1. > max_brush {
            self.brush_size = max_brush;
        } else {
            self.brush_size += 1.;
        }
    }

    fn decrease_brush(&mut self) {
        let min_brush = 1.0;
        if self.brush_size - 1. < min_brush {
            self.brush_size = min_brush;
        } else {
            self.brush_size -= 1.;
        }
    }

    fn clear_strokes(&mut self) {
        std::mem::take(&mut self.strokes);
    }

    fn undo_stroke(&mut self) {
        self.strokes.pop();
    }

    fn update(&mut self, touch: Touch) {
        let Touch {
            force,
            phase,
            location,
            inverted,
            ..
        } = touch;

        let inverted_str = if inverted { " (inverted) " } else { " " };
        let location_str = format!("{:.02},{:.02}", location.x, location.y);
        let stroke_str = format!("{location_str}{inverted_str}{:?}", self.style);

        let pressure = match force {
            Some(Force::Normalized(force)) => force,

            Some(Force::Calibrated {
                force,
                max_possible_force,
                altitude_angle: _,
            }) => force / max_possible_force,

            _ => 0.0,
        };

        let state = match phase {
            TouchPhase::Started => {
                println!("start stroke {stroke_str}");

                StylusState {
                    pos: StylusPosition::Down,
                    inverted,
                }
            }

            TouchPhase::Moved => {
                if self.stylus.down() {
                    print!("\r             {stroke_str}");
                    std::io::stdout().flush().unwrap();
                }

                self.stylus.state.inverted = inverted;
                self.stylus.state
            }

            TouchPhase::Ended | TouchPhase::Cancelled => {
                println!("\rend stroke   {stroke_str}\n");

                StylusState {
                    pos: StylusPosition::Up,
                    inverted,
                }
            }
        };

        self.stylus.pos = location;
        self.stylus.pressure = pressure;
        self.stylus.state = state;

        self.handle_update(phase);
    }

    fn handle_update(&mut self, phase: TouchPhase) {
        if self.stylus.inverted() {
            if phase == TouchPhase::Moved && self.stylus.down() {
                for stroke in self.strokes.iter_mut() {
                    'inner: for point in stroke.points.iter() {
                        let dist = ((self.stylus.pos.x - point.pos.x).powi(2)
                            + (self.stylus.pos.y - point.pos.y).powi(2))
                        .sqrt();
                        if dist < self.brush_size {
                            stroke.erased = true;
                            break 'inner;
                        }
                    }
                }
            }
        } else {
            match phase {
                TouchPhase::Started => {
                    self.strokes.push(Stroke {
                        points: Vec::new(),
                        color: rand::random(),
                        brush_size: self.brush_size,
                        style: self.style,
                        erased: false,
                    });
                }

                TouchPhase::Moved => {
                    if let Some(stroke) = self.strokes.last_mut() {
                        if self.stylus.down() {
                            stroke.points.push(Point {
                                pos: self.stylus.pos,
                                pressure: self.stylus.pressure,
                            });
                        }
                    }
                }

                TouchPhase::Ended | TouchPhase::Cancelled => {}
            };
        }
    }

    fn draw_strokes(&self, frame: &mut [u8], width: usize, height: usize) {
        for stroke in self.strokes.iter() {
            if !stroke.erased {
                (match if self.use_individual_style {
                    stroke.style
                } else {
                    self.style
                } {
                    StrokeStyle::Lines => lines,
                    StrokeStyle::Circles => circles,
                    StrokeStyle::CirclesPressure => circles_pressure,
                    StrokeStyle::Points => points,
                })(stroke, frame, width, height);
            }
        }
    }
}

fn print_human_info(identifier: &str) -> String {
    let identifier_cstr = CString::new(&identifier[..identifier.len() - 1]).expect("cstr");
    let api = hidapi::HidApi::new().unwrap();
    let device = api.open_path(&identifier_cstr).expect("open_path");
    let get_product_string = device.get_product_string();
    let get_manufacturer_string = device.get_manufacturer_string();
    let get_serial_number_string = device.get_serial_number_string();
    format!("product: {get_product_string:?}\nmanufacturer: {get_manufacturer_string:?}\nserial number: {get_serial_number_string:?}\n")
}

fn enumerate_devices<T>(ev: &EventLoop<T>) -> String {
    let mut devices = String::new();
    HidId::enumerate(ev).for_each(|id| {
        let identifier = id.persistent_identifier().unwrap();
        devices += &format!("{id:?} {identifier:?}\n");
        devices += &print_human_info(&identifier);
        devices += "\n";
    });
    KeyboardId::enumerate(ev).for_each(|id| {
        let identifier = id.persistent_identifier().unwrap();
        devices += &format!("{id:?} {identifier:?}\n");
        devices += &print_human_info(&identifier);
        devices += "\n";
    });
    MouseId::enumerate(ev).for_each(|id| {
        let identifier = id.persistent_identifier().unwrap();
        devices += &format!("{id:?} {identifier:?}\n");
        devices += &print_human_info(&identifier);
        devices += "\n";
    });
    GamepadHandle::enumerate(ev).for_each(|id| {
        let identifier = id.persistent_identifier().unwrap();
        devices += &format!("{id:?} {identifier:?}\n");
        devices += &print_human_info(&identifier);
        devices += "\n";
    });
    devices
}

fn new_pixels(window: &Window) -> Pixels {
    let size = window.inner_size();
    let tex = SurfaceTexture::new(size.width, size.height, &window);
    Pixels::new(size.width, size.height, tex).unwrap()
}

#[inline]
fn put_pixel(frame: &mut [u8], width: usize, height: usize, x: usize, y: usize, color: Color) {
    if x < width && y < height {
        let yw4 = y * width * 4;
        let x4 = x * 4;
        let sum = (yw4 + x4) as usize;
        let r = sum;
        let g = sum + 1;
        let b = sum + 2;
        let a = sum + 3;

        if a < frame.len() {
            frame[r] = color[0];
            frame[g] = color[1];
            frame[b] = color[2];
            frame[a] = 0xff;
        }
    }
}

//  fill_circle {{{
fn fill_circle(
    frame: &mut [u8],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    color: Color,
    radius: f64,
) {
    let x = x as isize;
    let y = y as isize;
    let mut dx = radius as isize;
    let mut dy = 0;
    let mut err = 1 - dx;
    while dx >= dy {
        for scan_x in (x - dy)..(x + dy) {
            put_pixel(
                frame,
                width,
                height,
                scan_x as usize,
                (y - dx) as usize,
                color,
            );
        }

        for scan_x in (x - dx)..(x + dx) {
            put_pixel(
                frame,
                width,
                height,
                scan_x as usize,
                (y + dy) as usize,
                color,
            );
        }

        for scan_x in (x - dx)..(x + dx) {
            put_pixel(
                frame,
                width,
                height,
                scan_x as usize,
                (y - dy) as usize,
                color,
            );
        }

        for scan_x in (x - dy)..(x + dy) {
            put_pixel(
                frame,
                width,
                height,
                scan_x as usize,
                (y + dx) as usize,
                color,
            );
        }

        dy += 1;
        if err < 0 {
            err = err + 2 * dy + 1;
        } else {
            dx = dx - 1;
            err = err + 2 * (dy - dx) + 1;
        }
    }
}
// }}}

// put_circle {{{
fn put_circle(
    frame: &mut [u8],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    color: Color,
    radius: f64,
) {
    let x = x as isize;
    let y = y as isize;
    let mut dx = radius as isize;
    let mut dy = 0;
    let mut err = 1 - dx;
    while dx >= dy {
        put_pixel(
            frame,
            width,
            height,
            (x + dx) as usize,
            (y + dy) as usize,
            color,
        );
        put_pixel(
            frame,
            width,
            height,
            (x - dx) as usize,
            (y + dy) as usize,
            color,
        );
        put_pixel(
            frame,
            width,
            height,
            (x + dx) as usize,
            (y - dy) as usize,
            color,
        );
        put_pixel(
            frame,
            width,
            height,
            (x - dx) as usize,
            (y - dy) as usize,
            color,
        );
        put_pixel(
            frame,
            width,
            height,
            (x + dy) as usize,
            (y + dx) as usize,
            color,
        );
        put_pixel(
            frame,
            width,
            height,
            (x - dy) as usize,
            (y + dx) as usize,
            color,
        );
        put_pixel(
            frame,
            width,
            height,
            (x + dy) as usize,
            (y - dx) as usize,
            color,
        );
        put_pixel(
            frame,
            width,
            height,
            (x - dy) as usize,
            (y - dx) as usize,
            color,
        );
        dy += 1;
        if err < 0 {
            err = err + 2 * dy + 1;
        } else {
            dx = dx - 1;
            err = err + 2 * (dy - dx) + 1;
        }
    }
}
// }}}

// circles {{{
fn circles(stroke: &Stroke, frame: &mut [u8], width: usize, height: usize) {
    let mut iter = stroke.points.windows(2);
    while let Some([a, b]) = iter.next() {
        let mut ax = a.pos.x as isize;
        let bx = b.pos.x as isize;
        let mut ay = a.pos.y as isize;
        let by = b.pos.y as isize;

        let dx = (bx - ax).abs();
        let sx = if ax < bx { 1 } else { -1 };
        let dy = -(by - ay).abs();
        let sy = if ay < by { 1 } else { -1 };
        let mut error = dx + dy;

        loop {
            fill_circle(
                frame,
                width,
                height,
                ax.try_into().unwrap_or(0),
                ay.try_into().unwrap_or(0),
                stroke.color,
                stroke.brush_size,
            );

            if ax == bx && ay == by {
                break;
            }

            let e2 = 2 * error;
            if e2 >= dy {
                if ax == bx {
                    break;
                }
                error += dy;
                ax += sx;
            }
            if e2 <= dx {
                if ay == by {
                    break;
                }
                error += dx;
                ay += sy;
            }
        }
    }
}
// }}}

// circles_pressure {{{
fn circles_pressure(stroke: &Stroke, frame: &mut [u8], width: usize, height: usize) {
    let mut iter = stroke.points.windows(2);
    while let Some([a, b]) = iter.next() {
        let mut ax = a.pos.x as isize;
        let bx = b.pos.x as isize;
        let mut ay = a.pos.y as isize;
        let by = b.pos.y as isize;

        let dx = (bx - ax).abs();
        let sx = if ax < bx { 1 } else { -1 };
        let dy = -(by - ay).abs();
        let sy = if ay < by { 1 } else { -1 };
        let mut error = dx + dy;

        let mut num_loops = 0;
        loop {
            if ax == bx && ay == by {
                break;
            }

            let e2 = 2 * error;
            if e2 >= dy {
                if ax == bx {
                    break;
                }
                error += dy;
                ax += sx;
            }
            if e2 <= dx {
                if ay == by {
                    break;
                }
                error += dx;
                ay += sy;
            }
            num_loops += 1;
        }

        ax = a.pos.x as isize;
        ay = a.pos.y as isize;
        error = dx + dy;
        let dp = (a.pressure - b.pressure) / num_loops as f64;
        let mut pressure = a.pressure;
        loop {
            fill_circle(
                frame,
                width,
                height,
                ax.try_into().unwrap_or(0),
                ay.try_into().unwrap_or(0),
                stroke.color,
                pressure * stroke.brush_size,
            );
            pressure += dp;

            if ax == bx && ay == by {
                break;
            }

            let e2 = 2 * error;
            if e2 >= dy {
                if ax == bx {
                    break;
                }
                error += dy;
                ax += sx;
            }
            if e2 <= dx {
                if ay == by {
                    break;
                }
                error += dx;
                ay += sy;
            }
        }
    }
}
// }}}

// lines {{{
fn lines(stroke: &Stroke, frame: &mut [u8], width: usize, height: usize) {
    let mut iter = stroke.points.windows(2);
    while let Some([a, b]) = iter.next() {
        let mut ax = a.pos.x as isize;
        let bx = b.pos.x as isize;
        let mut ay = a.pos.y as isize;
        let by = b.pos.y as isize;

        let dx = (bx - ax).abs();
        let sx = if ax < bx { 1 } else { -1 };
        let dy = -(by - ay).abs();
        let sy = if ay < by { 1 } else { -1 };
        let mut error = dx + dy;

        loop {
            put_pixel(
                frame,
                width,
                height,
                ax.try_into().unwrap_or(0),
                ay.try_into().unwrap_or(0),
                stroke.color,
            );

            if ax == bx && ay == by {
                break;
            }

            let e2 = 2 * error;
            if e2 >= dy {
                if ax == bx {
                    break;
                }
                error += dy;
                ax += sx;
            }
            if e2 <= dx {
                if ay == by {
                    break;
                }
                error += dx;
                ay += sy;
            }
        }
    }
}
// }}}

// points {{{
fn points(stroke: &Stroke, frame: &mut [u8], width: usize, height: usize) {
    for point in stroke.points.iter() {
        put_pixel(
            frame,
            width,
            height,
            point.pos.x as usize,
            point.pos.y as usize,
            stroke.color,
        );
    }
}
// }}}

fn clear(frame: &mut [u8]) {
    for pixel in frame.chunks_exact_mut(4) {
        pixel[0] = 0x00;
        pixel[1] = 0x00;
        pixel[2] = 0x00;
        pixel[3] = 0xff;
    }
}

#[allow(unreachable_code)]
fn main() {
    //windows::do_stuff().unwrap();
    //todo!();

    let ev = EventLoop::new();
    let window = WindowBuilder::new().build(&ev).unwrap();
    let device_str = enumerate_devices(&ev);

    let mut pixels = new_pixels(&window);

    let mut cursor_visible = true;
    let mut cursor_pos = Default::default();
    let mut state = State::default();
    state.init();
    println!("stroke style {:?}", state.style);

    ev.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput(KeyboardInput {
                        virtual_keycode: Some(key),
                        state: key_state,
                        ..
                    }),
                ..
            } => {
                use VirtualKeyCode::*;
                state.key(key, key_state);

                if state.just_pressed(Escape) {
                    *control_flow = ControlFlow::Exit;
                }

                if state.just_pressed(C) {
                    state.clear_strokes();
                    window.request_redraw();
                }

                if state.just_pressed(D) {
                    println!("{device_str}");
                }

                if state.just_pressed(F) {
                    state.fill_brush_head = !state.fill_brush_head;
                    window.request_redraw();
                }

                if state.control() && state.just_pressed(Z) {
                    state.undo_stroke();
                    window.request_redraw();
                }

                if state.just_pressed(R) && !state.shift() {
                    state.rotate_style();
                    window.request_redraw();
                    println!("stroke style {:?}", state.style);
                }

                if state.just_pressed(R) && state.shift() {
                    state.use_individual_style = !state.use_individual_style;
                    window.request_redraw();
                }

                if state.just_pressed(S) {
                    let num_string = std::fs::read_to_string("img/num.txt").expect("read num.txt");
                    let num = num_string.trim().parse::<usize>().expect("parse num.txt");
                    let filename = format!("img/strokes{num}.png");
                    let PhysicalSize { width, height } = window.inner_size();
                    let frame = pixels.get_frame();
                    clear(frame);
                    state.draw_strokes(frame, width as usize, height as usize);
                    image::save_buffer(&filename, frame, width, height, image::ColorType::Rgba8)
                        .expect(&format!("save {filename}"));
                    let next_num = num + 1;
                    std::fs::write("img/num.txt", format!("{next_num}")).expect("write num.txt");
                    println!("wrote image as {filename}");
                    window.request_redraw();
                }
            }

            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }

            Event::WindowEvent {
                event: WindowEvent::Touch(touch),
                ..
            } => {
                cursor_visible = false;
                window.set_cursor_visible(cursor_visible);
                state.update(touch);
                window.request_redraw();
            }

            Event::WindowEvent {
                event: WindowEvent::MouseWheel { delta, .. },
                ..
            } => {
                match delta {
                    MouseScrollDelta::LineDelta(_, y) if y.is_sign_positive() => {
                        state.increase_brush()
                    }
                    MouseScrollDelta::PixelDelta(pos) if pos.y.is_sign_positive() => {
                        state.increase_brush()
                    }
                    MouseScrollDelta::LineDelta(_, y) if y.is_sign_negative() => {
                        state.decrease_brush()
                    }
                    MouseScrollDelta::PixelDelta(pos) if pos.y.is_sign_negative() => {
                        state.decrease_brush()
                    }
                    _ => unreachable!(),
                };
                window.request_redraw();
            }

            Event::WindowEvent {
                event: WindowEvent::ReceivedCharacter(c @ ('[' | ']')),
                ..
            } => {
                match c {
                    '[' => state.decrease_brush(),
                    ']' => state.increase_brush(),
                    _ => unreachable!(),
                };
                window.request_redraw();
            }

            Event::MouseEvent(
                _,
                event @ (MouseEvent::MovedRelative(_, _) | MouseEvent::MovedAbsolute(_)),
            ) => {
                match event {
                    MouseEvent::MovedAbsolute(new_pos) => cursor_pos = new_pos,
                    MouseEvent::MovedRelative(x, y) => {
                        cursor_pos.x += x;
                        cursor_pos.y += y;
                    }
                    _ => unreachable!(),
                }

                if !cursor_visible {
                    cursor_visible = true;
                    window.set_cursor_visible(cursor_visible);
                    window.request_redraw();
                }
            }

            Event::RedrawRequested(_) => {
                let frame = pixels.get_frame();
                clear(frame);

                let PhysicalSize { width, height } = window.inner_size();
                let (width, height) = (width as usize, height as usize);

                state.draw_strokes(frame, width, height);

                if !cursor_visible {
                    (if state.fill_brush_head {
                        fill_circle
                    } else {
                        put_circle
                    })(
                        frame,
                        width,
                        height,
                        state.stylus.pos.x as usize,
                        state.stylus.pos.y as usize,
                        match (state.stylus.inverted(), state.stylus.down()) {
                            (true, true) => [0xfa, 0x34, 0x33],
                            (true, false) => [0x53, 0x11, 0x11],
                            (false, true) => [0xff, 0xff, 0xff],
                            (false, false) => [0x55, 0x55, 0x55],
                        },
                        state.brush_size,
                    );
                }

                pixels.render().unwrap();
            }

            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => {
                pixels = new_pixels(&window);
                window.request_redraw();
            }

            _ => {}
        }
    });
}
