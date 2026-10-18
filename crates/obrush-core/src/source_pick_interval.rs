//! Private outward interval evidence for bounded dense read-only picking.
//! Uncertain arithmetic is not a miss. Scalar authority order mirrors frozen
//! Glam0.30.10 operations and the unchanged OBrush EPS1e-15 triangle predicate.
const EPS: f64 = 1e-15;
const ONE_PLUS_EPS: f64 = 1.0 + EPS;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Interval {
    Finite { lo: f64, hi: f64 },
    Whole,
}

pub fn next_up(x: f64) -> f64 {
    if x.is_nan() || x == f64::INFINITY {
        return x;
    }
    if x == 0.0 {
        return f64::from_bits(1);
    }
    let bits = x.to_bits();
    f64::from_bits(if x > 0.0 { bits + 1 } else { bits - 1 })
}
pub fn next_down(x: f64) -> f64 {
    if x.is_nan() || x == f64::NEG_INFINITY {
        return x;
    }
    if x == 0.0 {
        return -f64::from_bits(1);
    }
    let bits = x.to_bits();
    f64::from_bits(if x > 0.0 { bits - 1 } else { bits + 1 })
}

impl Interval {
    pub fn point(x: f64) -> Self {
        Self::range(x, x)
    }
    pub fn range(lo: f64, hi: f64) -> Self {
        if lo.is_finite() && hi.is_finite() && lo <= hi {
            Self::Finite { lo, hi }
        } else {
            Self::Whole
        }
    }
    fn outward(lo: f64, hi: f64) -> Self {
        Self::range(next_down(lo), next_up(hi))
    }
    #[cfg(test)]
    pub fn contains(self, x: f64) -> bool {
        match self {
            Self::Finite { lo, hi } => lo <= x && x <= hi,
            Self::Whole => true,
        }
    }
    pub fn bounds(self) -> Option<(f64, f64)> {
        match self {
            Self::Finite { lo, hi } => Some((lo, hi)),
            Self::Whole => None,
        }
    }
    pub fn plus(self, rhs: Self) -> Self {
        match (self.bounds(), rhs.bounds()) {
            (Some((lo, hi)), Some((rlo, rhi))) => Self::outward(lo + rlo, hi + rhi),
            _ => Self::Whole,
        }
    }
    pub fn minus(self, rhs: Self) -> Self {
        match (self.bounds(), rhs.bounds()) {
            (Some((lo, hi)), Some((rlo, rhi))) => Self::outward(lo - rhi, hi - rlo),
            _ => Self::Whole,
        }
    }
    pub fn neg(self) -> Self {
        match self.bounds() {
            Some((lo, hi)) => Self::range(-hi, -lo),
            None => Self::Whole,
        }
    }
    pub fn times(self, rhs: Self) -> Self {
        match (self.bounds(), rhs.bounds()) {
            (Some((lo, hi)), Some((rlo, rhi))) => {
                let values = [lo * rlo, lo * rhi, hi * rlo, hi * rhi];
                if values.iter().any(|x| !x.is_finite()) {
                    return Self::Whole;
                }
                let min = values.iter().copied().fold(f64::INFINITY, f64::min);
                let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                Self::outward(min, max)
            }
            _ => Self::Whole,
        }
    }
    pub fn quotient(self, rhs: Self) -> Self {
        match (self.bounds(), rhs.bounds()) {
            (Some((lo, hi)), Some((rlo, rhi))) if rlo > 0.0 || rhi < 0.0 => {
                let values = [lo / rlo, lo / rhi, hi / rlo, hi / rhi];
                if values.iter().any(|x| !x.is_finite()) {
                    return Self::Whole;
                }
                let min = values.iter().copied().fold(f64::INFINITY, f64::min);
                let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                Self::outward(min, max)
            }
            _ => Self::Whole,
        }
    }
    pub fn abs_max(self) -> Option<f64> {
        self.bounds().map(|(lo, hi)| lo.abs().max(hi.abs()))
    }
    pub fn abs_min(self) -> Option<f64> {
        self.bounds().and_then(|(lo, hi)| {
            if lo > 0.0 {
                Some(lo)
            } else if hi < 0.0 {
                Some(-hi)
            } else {
                None
            }
        })
    }
    pub fn hull(self, rhs: Self) -> Self {
        match (self.bounds(), rhs.bounds()) {
            (Some((lo, hi)), Some((rlo, rhi))) => Self::range(lo.min(rlo), hi.max(rhi)),
            _ => Self::Whole,
        }
    }
    pub fn intersect(self, rhs: Self) -> Self {
        match (self.bounds(), rhs.bounds()) {
            (Some((lo, hi)), Some((rlo, rhi))) => Self::range(lo.max(rlo), hi.min(rhi)),
            (left_bounds, None) => match left_bounds {
                Some((lo, hi)) => Self::range(lo, hi),
                None => Self::Whole,
            },
            (None, Some((lo, hi))) => Self::range(lo, hi),
        }
    }
    fn inflated(self, error: Self) -> Self {
        match (self.bounds(), error.bounds()) {
            (Some((lo, hi)), Some((_, e))) if e >= 0.0 => Self::outward(lo - e, hi + e),
            _ => Self::Whole,
        }
    }
    fn round_error(self) -> Self {
        let Some(m) = self.abs_max() else {
            return Self::Whole;
        };
        let gap = (next_up(m) - m).max(m - next_down(m));
        Self::range(0.0, next_up(gap))
    }
}

// Nonnegative error bounds use these outward interval operations. A Whole
// error propagates Unknown rather than being clamped into a finite padding.
#[derive(Clone, Copy, Debug)]
pub struct Tracked {
    pub real: Interval,
    pub floating: Interval,
    pub error: Interval,
}
impl Tracked {
    pub fn input(range: Interval) -> Self {
        Self {
            real: range,
            floating: range,
            error: Interval::point(0.0),
        }
    }
    pub fn plus(self, rhs: Self) -> Self {
        let floating = self.floating.plus(rhs.floating);
        Self {
            real: self.real.plus(rhs.real),
            floating,
            error: self.error.plus(rhs.error).plus(floating.round_error()),
        }
    }
    pub fn minus(self, rhs: Self) -> Self {
        let floating = self.floating.minus(rhs.floating);
        Self {
            real: self.real.minus(rhs.real),
            floating,
            error: self.error.plus(rhs.error).plus(floating.round_error()),
        }
    }
    pub fn times(self, rhs: Self) -> Self {
        let floating = self.floating.times(rhs.floating);
        let l = self.real.abs_max().map_or(Interval::Whole, Interval::point);
        let r = rhs.real.abs_max().map_or(Interval::Whole, Interval::point);
        let error = l
            .times(rhs.error)
            .plus(r.times(self.error))
            .plus(self.error.times(rhs.error))
            .plus(floating.round_error());
        Self {
            real: self.real.times(rhs.real),
            floating,
            error,
        }
    }
    pub fn quotient(self, rhs: Self) -> Self {
        let real = self.real.quotient(rhs.real);
        let floating = self.floating.quotient(rhs.floating);
        let error = match (
            rhs.real.abs_min(),
            rhs.floating.abs_min(),
            self.real.abs_max(),
        ) {
            (Some(r), Some(f), Some(n)) => self
                .error
                .quotient(Interval::point(f))
                .plus(
                    Interval::point(n)
                        .times(rhs.error)
                        .quotient(Interval::point(r).times(Interval::point(f))),
                )
                .plus(floating.round_error()),
            _ => Interval::Whole,
        };
        Self {
            real,
            floating,
            error,
        }
    }
    fn tighten_real(mut self, proved: Interval) -> Self {
        self.real = self.real.intersect(proved);
        self.floating = self.floating.intersect(self.real.inflated(self.error));
        self
    }
}

type Triple = [Tracked; 3];
fn inputs(values: [Interval; 3]) -> Triple {
    values.map(Tracked::input)
}
fn cross(a: Triple, b: Triple) -> Triple {
    [
        a[1].times(b[2]).minus(b[1].times(a[2])),
        a[2].times(b[0]).minus(b[2].times(a[0])),
        a[0].times(b[1]).minus(b[0].times(a[1])),
    ]
}
fn dot(a: Triple, b: Triple) -> Tracked {
    a[0].times(b[0])
        .plus(a[1].times(b[1]))
        .plus(a[2].times(b[2]))
}
fn real_cross(a: [Interval; 3], b: [Interval; 3]) -> [Interval; 3] {
    [
        a[1].times(b[2]).minus(b[1].times(a[2])),
        a[2].times(b[0]).minus(b[2].times(a[0])),
        a[0].times(b[1]).minus(b[0].times(a[1])),
    ]
}

#[derive(Clone, Copy, Debug)]
pub struct Descriptor {
    pub a: [Interval; 3],
    pub e1: [Interval; 3],
    pub e2: [Interval; 3],
    pub normal_real: [Interval; 3],
    pub vertex_box: [Interval; 3],
    pub edge_reconstruction_error: [Interval; 3],
}
const _: () = assert!(std::mem::size_of::<Descriptor>() <= 512);

impl Descriptor {
    pub fn hull(self, rhs: Self) -> Self {
        fn hull(a: [Interval; 3], b: [Interval; 3]) -> [Interval; 3] {
            std::array::from_fn(|i| a[i].hull(b[i]))
        }
        Self {
            a: hull(self.a, rhs.a),
            e1: hull(self.e1, rhs.e1),
            e2: hull(self.e2, rhs.e2),
            normal_real: hull(self.normal_real, rhs.normal_real),
            vertex_box: hull(self.vertex_box, rhs.vertex_box),
            edge_reconstruction_error: hull(
                self.edge_reconstruction_error,
                rhs.edge_reconstruction_error,
            ),
        }
    }
    /// One independently authored scalar triangle. A production aggregate must
    /// retain/enclose both quad halves and every original face descriptor.
    pub fn triangle(vertices: [[f64; 3]; 3]) -> Self {
        let [a, b, c] = vertices;
        let e1 = std::array::from_fn(|i| b[i] - a[i]);
        let e2 = std::array::from_fn(|i| c[i] - a[i]);
        let errors = std::array::from_fn(|i| {
            Interval::point(b[i])
                .minus(Interval::point(a[i]))
                .round_error()
                .plus(
                    Interval::point(c[i])
                        .minus(Interval::point(a[i]))
                        .round_error(),
                )
        });
        Self {
            a: a.map(Interval::point),
            e1: e1.map(Interval::point),
            e2: e2.map(Interval::point),
            normal_real: real_cross(e1.map(Interval::point), e2.map(Interval::point)),
            vertex_box: std::array::from_fn(|i| {
                Interval::range(a[i].min(b[i]).min(c[i]), a[i].max(b[i]).max(c[i]))
            }),
            edge_reconstruction_error: errors,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Replay {
    pub determinant: Tracked,
    pub u: Tracked,
    pub v: Tracked,
    pub uv_sum: Tracked,
    pub t: Tracked,
}
impl Replay {
    pub fn new(desc: &Descriptor, origin: [f64; 3], direction: [f64; 3]) -> Self {
        let a = inputs(desc.a);
        let e1 = inputs(desc.e1);
        let e2 = inputs(desc.e2);
        let d = inputs(direction.map(Interval::point));
        let o = inputs(origin.map(Interval::point));
        let p = cross(d, e2);
        let mut determinant = dot(e1, p);
        // Exact real triple-product identity; error propagation still follows
        // the original floating determinant grammar, not the reassociation.
        let normal_dot = dot(d, inputs(desc.normal_real)).real.neg();
        determinant = determinant.tighten_real(normal_dot);
        let inverse = Tracked::input(Interval::point(1.0)).quotient(determinant);
        let offset = std::array::from_fn(|i| o[i].minus(a[i]));
        let u = dot(offset, p).times(inverse);
        let q = cross(offset, e1);
        let v = dot(d, q).times(inverse);
        let uv_sum = u.plus(v);
        let t = dot(e2, q).times(inverse);
        Self {
            determinant,
            u,
            v,
            uv_sum,
            t,
        }
    }
    pub fn predicate_proves_miss(self) -> bool {
        if self
            .determinant
            .floating
            .bounds()
            .is_some_and(|(lo, hi)| lo > -EPS && hi < EPS)
        {
            return true;
        }
        if self
            .u
            .floating
            .bounds()
            .is_some_and(|(lo, hi)| hi < -EPS || lo > ONE_PLUS_EPS)
        {
            return true;
        }
        if self.v.floating.bounds().is_some_and(|(_, hi)| hi < -EPS) {
            return true;
        }
        if self
            .uv_sum
            .floating
            .bounds()
            .is_some_and(|(lo, _)| lo > ONE_PLUS_EPS)
        {
            return true;
        }
        self.t.floating.bounds().is_some_and(|(_, hi)| hi < 0.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    ProvedMiss,
    Possible,
    Unknown,
}
#[derive(Clone, Copy, Debug)]
pub struct Certificate {
    pub verdict: Verdict,
    #[cfg(test)]
    pub envelope: Option<[[f64; 2]; 3]>,
    pub replay: Replay,
}

fn slab_proves_miss(
    envelope: [[f64; 2]; 3],
    origin: [f64; 3],
    direction: [f64; 3],
) -> Option<bool> {
    let mut enter = 0.0_f64;
    let mut exit = f64::INFINITY;
    for i in 0..3 {
        if direction[i] == 0.0 {
            if origin[i] < envelope[i][0] || origin[i] > envelope[i][1] {
                return Some(true);
            }
            continue;
        }
        let lo = Interval::point(envelope[i][0])
            .minus(Interval::point(origin[i]))
            .quotient(Interval::point(direction[i]));
        let hi = Interval::point(envelope[i][1])
            .minus(Interval::point(origin[i]))
            .quotient(Interval::point(direction[i]));
        let (llo, lhi) = lo.bounds()?;
        let (hlo, hhi) = hi.bounds()?;
        enter = enter.max(llo.min(hlo));
        exit = exit.min(lhi.max(hhi));
        if exit < 0.0 || enter > exit {
            return Some(true);
        }
    }
    Some(false)
}

pub fn certificate(desc: &Descriptor, origin: [f64; 3], direction: [f64; 3]) -> Certificate {
    let replay = Replay::new(desc, origin, direction);
    if origin
        .iter()
        .chain(direction.iter())
        .any(|v| !v.is_finite())
        || direction.iter().all(|v| *v == 0.0)
    {
        return Certificate {
            verdict: Verdict::Unknown,
            #[cfg(test)]
            envelope: None,
            replay,
        };
    }
    if replay.predicate_proves_miss() {
        return Certificate {
            verdict: Verdict::ProvedMiss,
            #[cfg(test)]
            envelope: None,
            replay,
        };
    }
    let sum_error = Interval::range(0.0, next_up(next_up(4.0) - 4.0));
    // For accepted finite u/v, each is in[-EPS,2], and all negative barycentric
    // weights total<=3EPS+error(fl(u+v)); proof notes bind this premise.
    let negative_mass = Interval::point(3.0)
        .times(Interval::point(EPS))
        .plus(sum_error);
    let mut envelope = [[0.0; 2]; 3];
    for i in 0..3 {
        let e1 = desc.e1[i]
            .abs_max()
            .map_or(Interval::Whole, Interval::point);
        let e2 = desc.e2[i]
            .abs_max()
            .map_or(Interval::Whole, Interval::point);
        let residual = Interval::point(direction[i].abs())
            .times(replay.t.error)
            .plus(e1.times(replay.u.error))
            .plus(e2.times(replay.v.error));
        let extent = match desc.vertex_box[i].bounds() {
            Some((lo, hi)) => Interval::point(hi).minus(Interval::point(lo)),
            None => Interval::Whole,
        };
        let barycentric_error = negative_mass.times(extent);
        let reconstruct = Interval::point(2.0).times(desc.edge_reconstruction_error[i]);
        let point = Tracked::input(Interval::point(origin[i])).plus(
            Tracked::input(Interval::point(direction[i])).times(Tracked::input(replay.t.floating)),
        );
        let padding = residual
            .plus(barycentric_error)
            .plus(reconstruct)
            .plus(point.error);
        let expanded = desc.vertex_box[i].inflated(padding);
        let Some((lo, hi)) = expanded.bounds() else {
            return Certificate {
                verdict: Verdict::Unknown,
                #[cfg(test)]
                envelope: None,
                replay,
            };
        };
        envelope[i] = [lo, hi];
    }
    match slab_proves_miss(envelope, origin, direction) {
        Some(true) => Certificate {
            verdict: Verdict::ProvedMiss,
            #[cfg(test)]
            envelope: Some(envelope),
            replay,
        },
        Some(false) => Certificate {
            verdict: Verdict::Possible,
            #[cfg(test)]
            envelope: Some(envelope),
            replay,
        },
        None => Certificate {
            verdict: Verdict::Unknown,
            #[cfg(test)]
            envelope: Some(envelope),
            replay,
        },
    }
}

#[cfg(test)]
pub fn scalar_hit(vertices: [[f64; 3]; 3], origin: [f64; 3], direction: [f64; 3]) -> Option<f64> {
    fn minus(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
        std::array::from_fn(|i| a[i] - b[i])
    }
    fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
        [
            a[1] * b[2] - b[1] * a[2],
            a[2] * b[0] - b[2] * a[0],
            a[0] * b[1] - b[0] * a[1],
        ]
    }
    fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
        (a[0] * b[0]) + (a[1] * b[1]) + (a[2] * b[2])
    }
    let [a, b, c] = vertices;
    let e1 = minus(b, a);
    let e2 = minus(c, a);
    let p = cross(direction, e2);
    let determinant = dot(e1, p);
    if determinant > -EPS && determinant < EPS {
        return None;
    }
    let inverse = 1.0 / determinant;
    let offset = minus(origin, a);
    let u = dot(offset, p) * inverse;
    if !(-EPS..=ONE_PLUS_EPS).contains(&u) {
        return None;
    }
    let q = cross(offset, e1);
    let v = dot(direction, q) * inverse;
    if v < -EPS || u + v > ONE_PLUS_EPS {
        return None;
    }
    let t = dot(e2, q) * inverse;
    (t.is_finite() && t >= 0.0).then_some(t)
}

#[cfg(test)]
#[path = "source_pick_interval_tests.rs"]
mod tests;
