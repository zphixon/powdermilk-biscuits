use pmb_tess::*;

#[derive(Clone, Copy, Debug)]
struct Pos {
    x: f32,
    y: f32,
}

impl Point for Pos {
    fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    fn x(&self) -> f32 {
        self.x
    }

    fn y(&self) -> f32 {
        self.y
    }
}

fn main() {
    #[rustfmt::skip]
    let points = &[
        Pos { x: 8.148584, y: -3.3291578 },
        Pos { x: 8.148584, y: -3.3291578 },
        Pos { x: 8.461885, y: -3.1091926 },
        Pos { x: 9.155146, y: -2.269322 },
        Pos { x: 10.121714, y: -0.50959253 },
        Pos { x: 10.555004, y: 0.776875 },
        Pos { x: 11.094946, y: 2.5899289 },
        Pos { x: 11.094946, y: 2.5899289 },
    ];

    points.windows(3).for_each(|w| {
        let quad = w.quadratic();

        let steps = 10;
        for t in (0..=steps).map(|t| t as f32 / steps as f32) {
            let int = quad.interpolate(t);
            println!(
                "        StrokeElement{{x:{},y:{},pressure:1.}},",
                int.x, int.y
            );
        }

        println!();
    });
}
