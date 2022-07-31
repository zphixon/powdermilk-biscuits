pub trait Quadratic<P: Point> {
    fn quadratic(&self) -> Bezier2<P>;
}

impl<P: Point> Quadratic<P> for &[P] {
    fn quadratic(&self) -> Bezier2<P> {
        assert_eq!(3, self.len());
        Bezier2 {
            a: self[0],
            b: self[1],
            c: self[2],
        }
    }
}

pub trait Point: Clone + Copy {
    fn new(x: f32, y: f32) -> Self;
    fn x(&self) -> f32;
    fn y(&self) -> f32;
}

pub struct Bezier2<P: Point> {
    pub a: P,
    pub b: P,
    pub c: P,
}

pub struct Bezier3<P: Point> {
    pub a: P,
    pub b: P,
    pub c: P,
    pub d: P,
}

macro_rules! interpolate {
    ($whomst:ty, $which:ident, $($field:ident),* $(,)?) => {
        impl<P: Point> $whomst  {
            pub fn interpolate(&self, t: f32) -> P {
                P::new(
                    $which(t, $(<P as Point>::x(&self.$field)),*),
                    $which(t, $(<P as Point>::y(&self.$field)),*),
                )
            }
        }
    };
}

interpolate!(Bezier2<P>, bezier2, a, b, c);
interpolate!(Bezier3<P>, bezier3, a, b, c, d);

pub fn bezier2(t: f32, w1: f32, w2: f32, w3: f32) -> f32 {
    let t2 = t * t;
    let mt = 1. - t;
    let mt2 = mt * mt;
    mt2 * w1 + 2. * mt * t * w2 + t2 * w3
}

pub fn bezier3(t: f32, w1: f32, w2: f32, w3: f32, w4: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;
    let mt = 1. - t;
    let mt2 = mt * mt;
    let mt3 = mt2 * mt;
    mt3 * w1 + 3. * mt2 * t * w2 + 3. * mt * t2 * w3 + t3 * w4
}
