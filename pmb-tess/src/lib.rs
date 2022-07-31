pub trait ToBezier<P: Point> {
    fn quadratic(&self) -> Quadratic<P>;
    fn cubic(&self) -> Cubic<P>;
}

impl<P: Point, const N: usize> ToBezier<P> for &[P; N] {
    fn quadratic(&self) -> Quadratic<P> {
        self.as_slice().quadratic()
    }

    fn cubic(&self) -> Cubic<P> {
        self.as_slice().cubic()
    }
}

impl<P: Point> ToBezier<P> for &[P] {
    fn quadratic(&self) -> Quadratic<P> {
        assert_eq!(3, self.len());
        Quadratic {
            a: self[0],
            b: self[1],
            c: self[2],
        }
    }

    fn cubic(&self) -> Cubic<P> {
        assert_eq!(4, self.len());
        Cubic {
            a: self[0],
            b: self[1],
            c: self[2],
            d: self[3],
        }
    }
}

pub trait Point: Clone + Copy {
    fn new(x: f32, y: f32) -> Self;
    fn x(&self) -> f32;
    fn y(&self) -> f32;
    fn zero() -> Self {
        Self::new(0., 0.)
    }
}

pub trait Bezier<P: Point> {
    fn weighted_basis(&self, t: f32) -> P;
    fn casteljau(&self, t: f32) -> P;
    fn flatten(&self, segments: usize) -> Vec<P> {
        steps(segments).map(|t| self.casteljau(t)).collect()
    }
    fn derivative(&self, t: f32) -> P;
    fn tangent(&self, t: f32) -> P {
        self.derivative(t)
    }
    fn direction(&self, t: f32) -> P {
        let tan = self.tangent(t);
        let d = (tan.x().powi(2) + tan.y().powi(2)).sqrt();
        P::new(tan.x() / d, tan.y() / d)
    }
    fn normal(&self, t: f32) -> P {
        let dir = self.direction(t);
        P::new(-dir.y(), dir.x())
    }
}

#[derive(Clone)]
pub struct Quadratic<P: Point> {
    pub a: P,
    pub b: P,
    pub c: P,
}

#[derive(Clone)]
pub struct Cubic<P: Point> {
    pub a: P,
    pub b: P,
    pub c: P,
    pub d: P,
}

pub fn steps(steps: usize) -> impl Iterator<Item = f32> {
    (0..=steps).map(move |t| t as f32 / steps as f32)
}

impl<P: Point> Bezier<P> for Quadratic<P> {
    fn weighted_basis(&self, t: f32) -> P {
        let t2 = t * t;
        let mt = 1. - t;
        let mt2 = mt * mt;
        P::new(
            mt2 * self.a.x() + 2. * mt * t * self.b.x() + t2 * self.c.x(),
            mt2 * self.a.y() + 2. * mt * t * self.b.y() + t2 * self.c.y(),
        )
    }

    fn casteljau(&self, t: f32) -> P {
        let tn = 1. - t;
        let ab_x = self.a.x() * tn + self.b.x() * t;
        let ab_y = self.a.y() * tn + self.b.y() * t;
        let bc_x = self.b.x() * tn + self.c.x() * t;
        let bc_y = self.b.y() * tn + self.c.y() * t;
        let ab_bc_x = ab_x * tn + bc_x * t;
        let ab_bc_y = ab_y * tn + bc_y * t;
        P::new(ab_bc_x, ab_bc_y)
    }

    fn derivative(&self, t: f32) -> P {
        let dx = 2. * (1. - t) * (self.b.x() - self.a.x()) + 2. * t * (self.c.x() - self.b.x());
        let dy = 2. * (1. - t) * (self.b.y() - self.a.y()) + 2. * t * (self.c.y() - self.b.y());
        P::new(dx, dy)
    }
}

impl<P: Point> Cubic<P> {
    pub fn derivative2(&self, t: f32) -> P {
        let apx = 3. * (self.b.x() - self.a.x());
        let apy = 3. * (self.b.y() - self.a.y());
        let bpx = 3. * (self.c.x() - self.b.x());
        let bpy = 3. * (self.c.y() - self.b.y());
        let cpx = 3. * (self.d.x() - self.c.x());
        let cpy = 3. * (self.d.y() - self.c.y());
        let ddx = 2. * Self::basis(0, t) * (bpx - apx) + 2. * Self::basis(1, t) * (cpx - bpx);
        let ddy = 2. * Self::basis(0, t) * (bpy - apy) + 2. * Self::basis(1, t) * (cpy - bpy);
        P::new(ddx, ddy)
    }

    fn basis(k: usize, t: f32) -> f32 {
        assert!((0..=2).contains(&k));
        if k == 0 {
            (1. - t) * (1. - t)
        } else if k == 1 {
            1. - t
        } else {
            t * t
        }
    }
}

impl<P: Point> Bezier<P> for Cubic<P> {
    fn weighted_basis(&self, t: f32) -> P {
        let t2 = t * t;
        let t3 = t2 * t;
        let mt = 1. - t;
        let mt2 = mt * mt;
        let mt3 = mt2 * mt;
        P::new(
            mt3 * self.a.x()
                + 3. * mt2 * t * self.b.x()
                + 3. * mt * t2 * self.c.x()
                + t3 * self.d.x(),
            mt3 * self.a.y()
                + 3. * mt2 * t * self.b.y()
                + 3. * mt * t2 * self.c.y()
                + t3 * self.d.y(),
        )
    }

    fn casteljau(&self, t: f32) -> P {
        let tn = 1. - t;
        let ab_x = self.a.x() * tn + self.b.x() * t;
        let ab_y = self.a.y() * tn + self.b.y() * t;
        let bc_x = self.b.x() * tn + self.c.x() * t;
        let bc_y = self.b.y() * tn + self.c.y() * t;
        let cd_x = self.c.x() * tn + self.d.x() * t;
        let cd_y = self.c.y() * tn + self.d.y() * t;
        let ab_bc_x = ab_x * tn + bc_x * t;
        let ab_bc_y = ab_y * tn + bc_y * t;
        let bc_cd_x = bc_x * tn + cd_x * t;
        let bc_cd_y = bc_y * tn + cd_y * t;
        let x = ab_bc_x * tn + bc_cd_x * t;
        let y = ab_bc_y * tn + bc_cd_y * t;
        P::new(x, y)
    }

    fn derivative(&self, t: f32) -> P {
        let tn = 1. - t;
        let dx = 3. * tn * tn * (self.b.x() - self.a.x())
            + 3. * tn * t * (self.c.x() - self.b.x())
            + 3. * t * t * (self.d.x() - self.c.x());
        let dy = 3. * tn * tn * (self.b.y() - self.a.y())
            + 3. * tn * t * (self.c.y() - self.b.y())
            + 3. * t * t * (self.d.y() - self.c.y());
        P::new(dx, dy)
    }
}
