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

use pmb_tess::*;
use std::fmt::{Display, Formatter, Result as FmtResult};

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

fn main() {
    let segments = 30;

    bigpoints
        .flatten(segments)
        .iter()
        .for_each(|p| println!("({p})"));

    for rib in bigpoints.flat_ribs(segments, 20.) {
        println!("({rib})");
        std::io::stdin().read_line(&mut String::new()).unwrap();
    }
}
