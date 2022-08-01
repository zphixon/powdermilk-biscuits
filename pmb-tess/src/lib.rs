//! Powdermilk Biscuits curve and tesselation library

use std::ops::Index;

/// Create a bezier curve from a list of points.
///
/// These methods do not create any geometry by themselves.
pub trait ToBezier<P: Point> {
    /// Create a quadratic Bezier curve.
    ///
    /// Panics if the length of the collection is not 3.
    fn quadratic(&self) -> Quadratic<P>;

    /// Create a cubic bezier curve.
    ///
    /// Panics if the length of the collection is not 4.
    fn cubic(&self) -> Cubic<P>;
}

impl<P: Point> ToBezier<P> for Vec<P> {
    fn quadratic(&self) -> Quadratic<P> {
        self.as_slice().quadratic()
    }

    fn cubic(&self) -> Cubic<P> {
        self.as_slice().cubic()
    }
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

/// A point in two-dimensional space.
pub trait Point: Clone + Copy {
    fn new(x: f32, y: f32) -> Self;

    fn zero() -> Self {
        Self::new(0., 0.)
    }

    fn x(&self) -> f32;
    fn y(&self) -> f32;

    fn magnitude(&self) -> f32 {
        (self.x().powi(2) + self.y().powi(2)).sqrt()
    }

    fn unit(&self) -> Self {
        let d = self.magnitude();
        Self::new(self.x() / d, self.y() / d)
    }

    fn dot(&self, other: &impl Point) -> f32 {
        self.x() * other.x() + self.y() * other.y()
    }
}

/// A Bezier curve.
pub trait Bezier<P: Point> {
    /// Evaluate the curve at `t` using Bernstein polynomials.
    fn weighted_basis(&self, t: f32) -> P;

    /// Evalute the curve at `t` using de Casteljau's altorithm.
    fn casteljau(&self, t: f32) -> P;

    /// Flatten the curve into non-uniform segments.
    fn flatten(&self, segments: usize) -> Vec<P> {
        steps(segments).map(|t| self.casteljau(t)).collect()
    }

    /// Evaluate the first derivative at `t`.
    fn derivative(&self, t: f32) -> P;

    /// Evaluate the tangent at `t`.
    ///
    /// This is equivalent to [derivative].
    fn tangent(&self, t: f32) -> P {
        self.derivative(t)
    }

    /// Evaluate the direction unit vector at `t`.
    fn direction(&self, t: f32) -> P {
        let tan = self.tangent(t);
        let d = tan.magnitude();
        P::new(tan.x() / d, tan.y() / d)
    }

    /// Evaluate the normal unit vector at `t`.
    fn normal(&self, t: f32) -> P {
        let dir = self.direction(t);
        P::new(-dir.y(), dir.x())
    }
}

/// A quadratic Bezier curve.
#[derive(Clone)]
pub struct Quadratic<P: Point> {
    pub a: P,
    pub b: P,
    pub c: P,
}

/// A cubic Bezier curve.
#[derive(Clone)]
pub struct Cubic<P: Point> {
    pub a: P,
    pub b: P,
    pub c: P,
    pub d: P,
}

/// Helper function that gives `steps` uniform steps between [0,1].
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
    /// The second derivative of the cubic Bezier curve.
    ///
    /// This is probably incorrect.
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

    // Bernstein polynomals for quadratic bezier curve
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

/// Cubic Hermite (Catmull-Rom) interpolator.
///
/// https://en.wikipedia.org/wiki/Cubic_Hermite_spline#Interpolation_on_the_unit_interval_with_matched_derivatives_at_endpoints
/// https://www.youtube.com/watch?v=9_aJGUTePYo
pub trait Hermite<P: Point>: Index<usize, Output = P> {
    /// The length of the collection.
    ///
    /// Must be >= 4.
    fn len(&self) -> usize;

    /// The indices into the collection which will be interpolated between.
    fn indices(&self, t: f32) -> (usize, usize, usize, usize) {
        assert!(self.len() >= 4);
        let p0 = t.trunc() as usize;
        let p1 = p0 + 1;
        let p2 = p1 + 1;
        let p3 = p2 + 1;
        (p0, p1, p2, p3)
    }

    /// The dot product of the interpolands and the transposed interpolation polynomial.
    fn dot(&self, t: f32, q1: f32, q2: f32, q3: f32, q4: f32) -> P {
        let (p0, p1, p2, p3) = self.indices(t);
        if p0 + 3 >= self.len() {
            return self[self.len() - 2];
        }
        let tx = self[p0].x() * q1 + self[p1].x() * q2 + self[p2].x() * q3 + self[p3].x() * q4;
        let ty = self[p0].y() * q1 + self[p1].y() * q2 + self[p2].y() * q3 + self[p3].y() * q4;
        P::new(0.5 * tx, 0.5 * ty)
    }

    /// Evaluate the spline at non-uniform `t`.
    fn interpolate(&self, t: f32) -> P {
        let u = t.fract();
        let uu = u * u;
        let uuu = uu * u;
        let q1 = -uuu + 2. * uu - u + 0.;
        let q2 = 3. * uuu - 5. * uu + 0. + 2.;
        let q3 = -3. * uuu + 4. * uu + u + 0.;
        let q4 = uuu - uu + 0. + 0.;
        self.dot(t, q1, q2, q3, q4)
    }

    /// Evaluate the first derivative at non-uniform `t`.
    fn derivative(&self, t: f32) -> P {
        let u = t.fract();
        let uu = u * u;
        let q1 = -3. * uu + 4. * u - 1.;
        let q2 = 9. * uu - 10. * u;
        let q3 = -9. * uu + 8. * u + 1.;
        let q4 = 3. * uu - 2. * u;
        self.dot(t, q1, q2, q3, q4)
    }

    fn rib(&self, t: f32, scale: f32) -> (P, P) {
        let (point, derivative) = (self.interpolate(t), self.derivative(t));
        let direction = derivative.unit();
        let normal = P::new(-direction.y() / scale, direction.x() / scale);

        (
            P::new(point.x() + normal.x(), point.y() + normal.y()),
            P::new(point.x() - normal.x(), point.y() - normal.y()),
        )
    }

    fn angle_change(&self, t1: f32, t2: f32) -> f32 {
        let a = self.derivative(t1).unit();
        let b = self.derivative(t2).unit();
        a.dot(&b) / (a.magnitude() * b.magnitude())
    }
}

impl<P: Point> Hermite<P> for [P] {
    fn len(&self) -> usize {
        self.as_ref().len()
    }
}
