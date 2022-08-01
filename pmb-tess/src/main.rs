#![allow(dead_code)]

#[allow(non_upper_case_globals)]
#[rustfmt::skip]
const bigpoints: &[Pos] = &[
    Pos { x: -3.3346415, y: 4.1671815, },
    Pos { x: -3.4107695, y: 4.1671815, },
    Pos { x: -3.4799757, y: 4.161991, },
    Pos { x: -3.5543728, y: 4.148496, },
    Pos { x: -3.6253088, y: 4.1246195, },
    Pos { x: -3.739501, y: 4.0696, },
    Pos { x: -3.813898, y: 4.018734, },
    Pos { x: -3.8709936, y: 3.9689047, },
    Pos { x: -3.9073267, y: 3.9138858, },
    Pos { x: -3.9678822, y: 3.8007324, },
    Pos { x: -3.9990263, y: 3.7031515, },
    Pos { x: -4.016327, y: 3.6180267, },
    Pos { x: -4.0249777, y: 3.5391312, },
    Pos { x: -4.0249777, y: 3.449855, },
    Pos { x: -4.011136, y: 3.2806442, },
    Pos { x: -3.9903746, y: 3.1197388, },
    Pos { x: -3.9540405, y: 2.9754431, },
    Pos { x: -3.9107876, y: 2.845681, },
    Pos { x: -3.853691, y: 2.7346036, },
    Pos { x: -3.8052464, y: 2.6608994, },
    Pos { x: -3.7533426, y: 2.6079562, },
    Pos { x: -3.7048979, y: 2.5685084, },
    Pos { x: -3.6426115, y: 2.5373654, },
    Pos { x: -3.582056, y: 2.5166037, },
    Pos { x: -3.5197697, y: 2.4999933, },
    Pos { x: -3.4574833, y: 2.4948027, },
    Pos { x: -3.4142294, y: 2.4948027, },
    Pos { x: -3.3796253, y: 2.505184, },
    Pos { x: -3.3623235, y: 2.5238693, },
    Pos { x: -3.353673, y: 2.547746, },
    Pos { x: -3.3571339, y: 2.582003, },
    Pos { x: -3.3830862, y: 2.6484418, },
    Pos { x: -3.4713252, y: 2.8301096, },
    Pos { x: -3.5768652, y: 3.006586, },
    Pos { x: -3.6772146, y: 3.1882539, },
    Pos { x: -3.7879457, y: 3.3730352, },
    Pos { x: -3.888294, y: 3.5526266, },
    Pos { x: -3.976533, y: 3.732218, },
    Pos { x: -4.0301685, y: 3.8630183, },
    Pos { x: -4.0699625, y: 3.9979713, },
    Pos { x: -4.090724, y: 4.095553, },
    Pos { x: -4.095914, y: 4.1775627, },
    Pos { x: -4.087264, y: 4.237772, },
    Pos { x: -4.0595818, y: 4.2990203, },
    Pos { x: -3.9851847, y: 4.4069824, },
    Pos { x: -3.8502314, y: 4.535707, },
    Pos { x: -3.6910563, y: 4.643669, },
    Pos { x: -3.511118, y: 4.7412505, },
    Pos { x: -3.3830862, y: 4.7973084, },
    Pos { x: -3.2515926, y: 4.855442, },
    Pos { x: -3.120101, y: 4.910461, },
];

use std::fmt::{Display, Formatter, Result as FmtResult};

use pmb_tess::*;

#[derive(Clone, Copy, Debug)]
struct Pos {
    pub x: f32,
    pub y: f32,
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

impl Display for Pos {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{:.05},{:.05}", self.x, self.y)
    }
}

fn as_storkeelment(int: Pos) {
    println!(
        "        StrokeElement{{x:{},y:{},pressure:1.}},",
        int.x, int.y
    );
}

#[rustfmt::skip]
fn print(c: &[Pos], bez: &impl Bezier<Pos>) {
    println!("\n\n\n\nimport matplotlib.pyplot as plt\nfig,ax=plt.subplots()\nax.grid(True)");
    print!("xp=["); c.iter().for_each(|p| print!("{:.05},", p.x())); println!("]");
    print!("yp=["); c.iter().for_each(|p| print!("{:.05},", p.y())); println!("]");

    let steps: Vec<Pos> = bez.flatten(32);
    print!("xc=["); steps.iter().for_each(|p| print!("{:.05},", p.x())); println!("]");
    print!("yc=["); steps.iter().for_each(|p| print!("{:.05},", p.y())); println!("]");

    let steps: Vec<Pos> = pmb_tess::steps(32).map(|t| bez.casteljau(t)).collect();
    print!("xw=["); steps.iter().for_each(|p| print!("{:.05},", p.x())); println!("]");
    print!("yw=["); steps.iter().for_each(|p| print!("{:.05},", p.y())); println!("]");

    let steps: Vec<Pos> = pmb_tess::steps(32).map(|t| bez.derivative(t)).collect();
    print!("xd=["); steps.iter().for_each(|p| print!("{:.05},", p.x())); println!("]");
    print!("yd=["); steps.iter().for_each(|p| print!("{:.05},", p.y())); println!("]");

    let steps: Vec<Pos> = pmb_tess::steps(32).map(|t| bez.direction(t)).collect();
    print!("xdd=["); steps.iter().for_each(|p| print!("{:.05},", p.x())); println!("]");
    print!("ydd=["); steps.iter().for_each(|p| print!("{:.05},", p.y())); println!("]");

    let steps: Vec<Pos> = pmb_tess::steps(32).map(|t| {
        let p = bez.casteljau(t);
        let n = bez.normal(t);
        Pos { x: p.x + n.x, y: p.y + n.y }
    }).collect();
    print!("xn=["); steps.iter().for_each(|p| print!("{:.05},", p.x())); println!("]");
    print!("yn=["); steps.iter().for_each(|p| print!("{:.05},", p.y())); println!("]");

    println!("ax.plot(xp, yp, c='black')");
    println!("ax.plot(xc, yc, c='firebrick')");
    println!("ax.plot(xw, yc, c='lightcoral')");
    println!("ax.plot(xd, yd, c='seagreen')");
    println!("ax.plot(xdd, ydd, c='palegreen')");
    println!("ax.plot(xn, yn, c='darkorchid')");
    println!("plt.show()\n\n\n\n");
}

fn main() {
    #[rustfmt::skip]
    let points = &[
        Pos { x: 8.148584, y: -3.3291578 },
        Pos { x: -9.155146, y: -2.269322 },
        Pos { x: 10.121714, y: 0.50959253 },
        Pos { x: 11.094946, y: 8.5899289 },
    ];
    let cub = points.cubic();
    let quad = (&points[1..]).quadratic();

    print(points, &cub);
    print(bigpoints, &quad);

    println!("\n\n\n\nimport matplotlib.pyplot as plt\nfig,ax=plt.subplots()\nax.grid(True)");

    print!("xp=[");
    bigpoints.iter().for_each(|p| print!("{:.05},", p.x()));
    println!("]");
    print!("yp=[");
    bigpoints.iter().for_each(|p| print!("{:.05},", p.y()));
    println!("]");
    println!("ax.plot(xp, yp, c='black')");

    let pts: Vec<_> = pmb_tess::steps(50)
        .map(|t| t * (bigpoints.len() - 2) as f32)
        .map(|t| bigpoints.interpolate(t))
        .collect();
    print!("x=[");
    pts.iter().for_each(|pt| print!("{:.05},", pt.x));
    println!("]");
    print!("y=[");
    pts.iter().for_each(|pt| print!("{:.05},", pt.y));
    println!("]");
    println!("ax.plot(x, y, c='lightcoral')");

    let pts: Vec<_> = pmb_tess::steps(50)
        .map(|t| t * (bigpoints.len() - 2) as f32)
        .map(|t| (bigpoints.derivative(t), bigpoints.interpolate(t)))
        .collect();

    for (deriv, interp) in pts {
        let dir = deriv.unit();
        let norm = Pos {
            x: -dir.y() / 20.,
            y: dir.x() / 20.,
        };

        let rib1 = Pos {
            x: interp.x + norm.x,
            y: interp.y + norm.y,
        };

        let rib2 = Pos {
            x: interp.x - norm.x,
            y: interp.y - norm.y,
        };

        println!("x=[{},{}]", rib1.x, rib2.x);
        println!("y=[{},{}]", rib1.y, rib2.y);
        println!("ax.plot(x,y,color='cornflowerblue')");
    }
    println!("plt.show()\n\n\n\n");

    println!("\n\n\n\nimport matplotlib.pyplot as plt\nfig,ax=plt.subplots()\nax.grid(True)");
    let press = [
        0.45214844, 0.4794922, 0.55078125, 0.55078125, 0.5839844, 0.6152344, 0.6435547, 0.6699219,
        0.69433594, 0.7207031, 0.7470703, 0.7734375, 0.7988281, 0.8232422, 0.8652344, 0.8652344,
        0.88378906, 0.9013672, 0.9189453, 0.9345703, 0.96484375, 0.96484375, 0.9785156, 0.9941406,
        1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1.,
        1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1.,
        1., 1., 1., 1., 1., 1., 0.953125, 0.89941406, 0.8330078, 0.7529297, 0.5019531, 0.33398438,
        0.22265625,
    ];

    let p: Vec<_> = press
        .iter()
        .enumerate()
        .map(|(i, p)| Pos { x: i as f32, y: *p })
        .collect();

    print!("x=[");
    p.iter().for_each(|p| print!("{},", p.x));
    println!("]");

    print!("p=[");
    p.iter().for_each(|p| print!("{},", p.y));
    println!("]");

    print!("a=[{}", press[0]);
    press.windows(3).for_each(|w| {
        if let &[a, b, c] = w {
            let avg = (a + b + c) / 3.;
            print!("{},", avg);
        } else {
            unreachable!()
        }
    });
    println!("{}]", press.last().unwrap());

    println!("ax.plot(x,p,c='lightcoral')\nax.plot(x,a,c='cornflowerblue')\nplt.show()\n\n\n");
}
