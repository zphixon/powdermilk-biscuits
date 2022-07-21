pub mod coords;

use crate::{
    graphics::coords::{PixelPos, StrokePos},
    Color, Stroke,
};

pub fn circle_points(radius: f32, num_segments: usize) -> Vec<f32> {
    let mut segments = Vec::with_capacity(num_segments);

    let mut angle = 0.0;
    let segments_f32 = num_segments as f32;
    for _ in 0..num_segments {
        let d_theta = std::f32::consts::TAU / segments_f32;
        angle += d_theta;
        let (x, y) = angle.sin_cos();
        segments.push(x * radius);
        segments.push(y * radius);
    }

    segments
}

#[inline]
pub fn clear(frame: &mut [u8]) {
    for pixel in frame.chunks_exact_mut(4) {
        pixel[0] = 0x00;
        pixel[1] = 0x00;
        pixel[2] = 0x00;
        pixel[3] = 0xff;
    }
}

#[inline]
pub fn put_pixel_stroke(
    frame: &mut [u8],
    width: usize,
    height: usize,
    pos: StrokePos,
    color: Color,
    zoom: f32,
    screen_in_paper: StrokePos,
) {
    put_pixel_absolute(
        frame,
        width,
        height,
        PixelPos::from_stroke(pos, zoom, screen_in_paper),
        color,
    );
}

#[inline]
pub fn put_pixel_absolute(
    frame: &mut [u8],
    width: usize,
    height: usize,
    pos: PixelPos,
    color: Color,
) {
    let PixelPos { x, y } = pos;
    if x < width as isize && y < height as isize && y >= 0 && x >= 0 {
        let yw4 = y * width as isize * 4;
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
    pos: StrokePos,
    color: Color,
    radius: f32,
    zoom: f32,
    screen_in_paper: StrokePos,
) {
    let PixelPos { x, y } = PixelPos::from_stroke(pos, zoom, screen_in_paper);
    let x = x as isize;
    let y = y as isize;
    let mut dx = radius as isize;
    let mut dy = 0;
    let mut err = 1 - dx;
    while dx >= dy {
        for scan_x in (x - dy)..(x + dy) {
            put_pixel_absolute(
                frame,
                width,
                height,
                PixelPos {
                    x: scan_x,
                    y: (y - dx),
                },
                color,
            );
        }

        for scan_x in (x - dx)..(x + dx) {
            put_pixel_absolute(
                frame,
                width,
                height,
                PixelPos {
                    x: scan_x,
                    y: (y + dy),
                },
                color,
            );
        }

        for scan_x in (x - dx)..(x + dx) {
            put_pixel_absolute(
                frame,
                width,
                height,
                PixelPos {
                    x: scan_x,
                    y: (y - dy),
                },
                color,
            );
        }

        for scan_x in (x - dy)..(x + dy) {
            put_pixel_absolute(
                frame,
                width,
                height,
                PixelPos {
                    x: scan_x,
                    y: (y + dx),
                },
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

pub fn fill_circle_absolute(
    frame: &mut [u8],
    width: usize,
    height: usize,
    pos: PixelPos,
    color: Color,
    radius: f32,
) {
    let PixelPos { x, y } = pos;
    let x = x as isize;
    let y = y as isize;
    let mut dx = radius as isize;
    let mut dy = 0;
    let mut err = 1 - dx;
    while dx >= dy {
        for scan_x in (x - dy)..(x + dy) {
            put_pixel_absolute(
                frame,
                width,
                height,
                PixelPos {
                    x: scan_x,
                    y: (y - dx),
                },
                color,
            );
        }

        for scan_x in (x - dx)..(x + dx) {
            put_pixel_absolute(
                frame,
                width,
                height,
                PixelPos {
                    x: scan_x,
                    y: (y + dy),
                },
                color,
            );
        }

        for scan_x in (x - dx)..(x + dx) {
            put_pixel_absolute(
                frame,
                width,
                height,
                PixelPos {
                    x: scan_x,
                    y: (y - dy),
                },
                color,
            );
        }

        for scan_x in (x - dy)..(x + dy) {
            put_pixel_absolute(
                frame,
                width,
                height,
                PixelPos {
                    x: scan_x,
                    y: (y + dx),
                },
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

pub fn put_circle_absolute(
    frame: &mut [u8],
    width: usize,
    height: usize,
    pos: PixelPos,
    color: Color,
    radius: f32,
) {
    let PixelPos { x, y } = pos;
    let x = x as isize;
    let y = y as isize;
    let mut dx = radius as isize;
    let mut dy = 0;
    let mut err = 1 - dx;
    while dx >= dy {
        put_pixel_absolute(
            frame,
            width,
            height,
            PixelPos {
                x: (x + dx),
                y: (y + dy),
            },
            color,
        );
        put_pixel_absolute(
            frame,
            width,
            height,
            PixelPos {
                x: (x - dx),
                y: (y + dy),
            },
            color,
        );
        put_pixel_absolute(
            frame,
            width,
            height,
            PixelPos {
                x: (x + dx),
                y: (y - dy),
            },
            color,
        );
        put_pixel_absolute(
            frame,
            width,
            height,
            PixelPos {
                x: (x - dx),
                y: (y - dy),
            },
            color,
        );
        put_pixel_absolute(
            frame,
            width,
            height,
            PixelPos {
                x: (x + dy),
                y: (y + dx),
            },
            color,
        );
        put_pixel_absolute(
            frame,
            width,
            height,
            PixelPos {
                x: (x - dy),
                y: (y + dx),
            },
            color,
        );
        put_pixel_absolute(
            frame,
            width,
            height,
            PixelPos {
                x: (x + dy),
                y: (y - dx),
            },
            color,
        );
        put_pixel_absolute(
            frame,
            width,
            height,
            PixelPos {
                x: (x - dy),
                y: (y - dx),
            },
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

pub fn circles(
    stroke: &Stroke,
    frame: &mut [u8],
    width: usize,
    height: usize,
    zoom: f32,
    screen_in_paper: StrokePos,
) {
    let mut iter = stroke.points.windows(2);
    while let Some([a, b]) = iter.next() {
        let PixelPos { x: ax, y: ay } =
            PixelPos::from_stroke(StrokePos { x: a.x, y: a.y }, zoom, screen_in_paper);
        let PixelPos { x: bx, y: by } =
            PixelPos::from_stroke(StrokePos { x: b.x, y: b.y }, zoom, screen_in_paper);

        let mut ax = ax as isize;
        let bx = bx as isize;
        let mut ay = ay as isize;
        let by = by as isize;

        let dx = (bx - ax).abs();
        let sx = if ax < bx { 1 } else { -1 };
        let dy = -(by - ay).abs();
        let sy = if ay < by { 1 } else { -1 };
        let mut error = dx + dy;

        loop {
            fill_circle_absolute(
                frame,
                width,
                height,
                PixelPos { x: ax, y: ay },
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

pub fn circles_pressure(
    stroke: &Stroke,
    frame: &mut [u8],
    width: usize,
    height: usize,
    zoom: f32,
    screen_in_paper: StrokePos,
) {
    let mut iter = stroke.points.windows(2);
    while let Some([a, b]) = iter.next() {
        let PixelPos { x: ax, y: ay } =
            PixelPos::from_stroke(StrokePos { x: a.x, y: a.y }, zoom, screen_in_paper);
        let PixelPos { x: bx, y: by } =
            PixelPos::from_stroke(StrokePos { x: b.x, y: b.y }, zoom, screen_in_paper);

        let mut ax = ax as isize;
        let bx = bx as isize;
        let mut ay = ay as isize;
        let by = by as isize;

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

        ax = ax as isize;
        ay = ay as isize;
        error = dx + dy;
        let dp = (a.pressure - b.pressure) / num_loops as f32;
        let mut pressure = a.pressure;
        loop {
            fill_circle_absolute(
                frame,
                width,
                height,
                PixelPos { x: ax, y: ay },
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

pub fn lines(
    stroke: &Stroke,
    frame: &mut [u8],
    width: usize,
    height: usize,
    zoom: f32,
    screen_in_paper: StrokePos,
) {
    let mut iter = stroke.points.windows(2);
    while let Some([a, b]) = iter.next() {
        let PixelPos { x: ax, y: ay } =
            PixelPos::from_stroke(StrokePos { x: a.x, y: a.y }, zoom, screen_in_paper);
        let PixelPos { x: bx, y: by } =
            PixelPos::from_stroke(StrokePos { x: b.x, y: b.y }, zoom, screen_in_paper);

        let mut ax = ax as isize;
        let bx = bx as isize;
        let mut ay = ay as isize;
        let by = by as isize;

        let dx = (bx - ax).abs();
        let sx = if ax < bx { 1 } else { -1 };
        let dy = -(by - ay).abs();
        let sy = if ay < by { 1 } else { -1 };
        let mut error = dx + dy;

        loop {
            put_pixel_absolute(
                frame,
                width,
                height,
                PixelPos {
                    x: ax.try_into().unwrap_or(0),
                    y: ay.try_into().unwrap_or(0),
                },
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

pub fn points(
    stroke: &Stroke,
    frame: &mut [u8],
    width: usize,
    height: usize,
    zoom: f32,
    screen_in_paper: StrokePos,
) {
    for point in stroke.points.iter() {
        put_pixel_stroke(
            frame,
            width,
            height,
            StrokePos {
                x: point.x,
                y: point.y,
            },
            stroke.color,
            zoom,
            screen_in_paper,
        );
    }
}

pub fn spline(
    stroke: &Stroke,
    frame: &mut [u8],
    width: usize,
    height: usize,
    zoom: f32,
    screen_in_paper: StrokePos,
) {
    if stroke.spline.is_none() {
        return;
    }

    let spline = stroke.spline.as_ref().expect("stroke should have spline");

    let (min, max) = spline.knot_domain();
    let dt = 0.001;

    let mut t = min;
    let mut px = 0;
    let mut py = 0;

    while t < max {
        let pos = spline.point(t);

        let PixelPos { x: sx, y: sy } = PixelPos::from_stroke(pos, zoom, screen_in_paper);

        if sx as isize != px || sy as isize != py {
            put_pixel_stroke(
                frame,
                width,
                height,
                pos,
                stroke.color,
                zoom,
                screen_in_paper,
            );
        }

        px = sx as isize;
        py = sy as isize;
        t += dt;
    }
}
