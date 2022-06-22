use crate::{Color, Stroke};

#[inline]
pub fn put_pixel(frame: &mut [u8], width: usize, height: usize, x: usize, y: usize, color: Color) {
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

pub fn fill_circle(
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

pub fn put_circle(
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

pub fn circles(stroke: &Stroke, frame: &mut [u8], width: usize, height: usize) {
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

pub fn circles_pressure(stroke: &Stroke, frame: &mut [u8], width: usize, height: usize) {
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
                (pressure * stroke.brush_size).max(1.0),
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

pub fn lines(stroke: &Stroke, frame: &mut [u8], width: usize, height: usize) {
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

pub fn points(stroke: &Stroke, frame: &mut [u8], width: usize, height: usize) {
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
