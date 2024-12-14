use core::ops::{Add, AddAssign, Neg, Sub, SubAssign};

use openvm_algebra_guest::Field;

use crate::{weierstrass::IntrinsicCurve, Group};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[repr(C)]
pub struct AffinePoint<F> {
    pub x: F,
    pub y: F,
}

impl<F: Field> AffinePoint<F>
where
    F: Neg<Output = F>,
    for<'a> &'a F: Add<&'a F, Output = F>,
    for<'a> &'a F: Sub<&'a F, Output = F>,
{
    pub fn new(x: F, y: F) -> Self {
        Self { x, y }
    }

    pub fn neg_borrow<'a>(&'a self) -> Self
    where
        &'a F: Neg<Output = F>,
    {
        Self {
            x: self.x.clone(),
            y: Neg::neg(&self.y),
        }
    }

    pub fn is_infinity(&self) -> bool {
        self.x == F::ZERO && self.y == F::ZERO
    }

    fn add_impl(&self, other: &Self) -> Self {
        if self.is_infinity() {
            return other.clone();
        }
        if other.is_infinity() {
            return self.clone();
        }

        if self.x == other.x {
            if self.y == other.y.clone().neg() {
                return Self::IDENTITY;
            }
            if self.y == other.y {
                return self.double();
            }
        }

        let lambda = (&other.y - &self.y).div_unsafe(&(&other.x - &self.x));
        let mut lambda_square = lambda.clone();
        lambda_square.square_assign();
        let x3 = lambda_square - &self.x - &other.x;
        let y3 = lambda * &(&self.x - &x3) - &self.y;

        // let mut lambda = other.y.clone();
        // lambda -= self.y.clone();
        // let mut denom = other.x.clone();
        // denom -= self.x.clone();
        // lambda *= denom;
        // lambda.div_assign_unsafe(&denom);

        // x3 = lambda^2 - x1 - x2
        // let mut x3 = lambda.clone();
        // x3.square_assign();
        // x3 -= self.x.clone();
        // x3 -= other.x.clone();

        // // y3 = lambda * (x1 - x3) - y1
        // let x1_minus_x3 = self.x.clone() - x3.clone();
        // let mut y3 = lambda;
        // y3 *= x1_minus_x3;
        // y3 -= self.y.clone();

        Self::new(x3, y3)
    }

    fn sub_impl(&self, other: &Self) -> Self {
        self.add_impl(&other.neg())
    }
}

impl<F: Field> Group for AffinePoint<F>
where
    F: Neg<Output = F>,
    for<'a> &'a F: Add<&'a F, Output = F>,
    for<'a> &'a F: Sub<&'a F, Output = F>,
{
    type SelfRef<'a>
        = &'a Self
    where
        Self: 'a;

    const IDENTITY: Self = Self {
        x: F::ZERO,
        y: F::ZERO,
    };

    fn is_identity(&self) -> bool {
        self.x == F::ZERO && self.y == F::ZERO
    }

    fn double(&self) -> Self {
        if self.is_identity() {
            self.clone()
        } else {
            self.clone() + self
        }
    }

    fn double_assign(&mut self) {
        if self.is_identity() {
            return;
        }
        *self = self.double();
    }
}

impl<F> Neg for AffinePoint<F>
where
    F: Neg<Output = F>,
{
    type Output = AffinePoint<F>;

    fn neg(self) -> AffinePoint<F> {
        Self {
            x: self.x,
            y: self.y.neg(),
        }
    }
}

impl<F: Field> Add for AffinePoint<F>
where
    F: Neg<Output = F>,
    for<'a> &'a F: Add<&'a F, Output = F>,
    for<'a> &'a F: Sub<&'a F, Output = F>,
{
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        self.add_impl(&rhs)
    }
}

impl<F: Field> Add<&AffinePoint<F>> for AffinePoint<F>
where
    F: Neg<Output = F>,
    for<'c> &'c F: Add<&'c F, Output = F>,
    for<'c> &'c F: Sub<&'c F, Output = F>,
{
    type Output = Self;

    fn add(self, rhs: &AffinePoint<F>) -> Self {
        self.add_impl(rhs)
    }
}

impl<'a, F: Field> Add for &'a AffinePoint<F>
where
    F: Neg<Output = F>,
    for<'b> &'b F: Add<&'b F, Output = F>,
    for<'b> &'b F: Sub<&'b F, Output = F>,
{
    type Output = AffinePoint<F>;

    fn add(self, rhs: &'a AffinePoint<F>) -> Self::Output {
        self.add_impl(&rhs.clone())
    }
}

// impl<F: Field> Add<&Self> for AffinePoint<F>
// where
//     F: Neg<Output = F>
//         + Add<Output = F>
//         + Sub<Output = F>
//         + for<'a> Add<&'a F, Output = F>
//         + for<'a> Sub<&'a F, Output = F>,
//     for<'a> &'a F: Add<&'a F, Output = F>,
//     for<'a> &'a F: Sub<&'a F, Output = F>,
// {
//     type Output = Self;

//     fn add(self, rhs: &Self) -> Self {
//         Self::add_impl(&self, rhs)
//     }
// }

// impl<F: Field> Add<&AffinePoint<F>> for &AffinePoint<F>
// where
//     F: Neg<Output = F>
//         + Add<Output = F>
//         + Sub<Output = F>
//         + for<'a> Add<&'a F, Output = F>
//         + for<'a> Sub<&'a F, Output = F>,
//         + for<'a> AddAssign<&'a F>
//         + for<'a> SubAssign<&'a F>,
//     for<'a> &'a F: Sub<&'a F, Output = F>,
// {
//     type Output = AffinePoint<F>;

//     fn add(self, rhs: &AffinePoint<F>) -> Self::Output {
//         self.add_impl(rhs)
//     }
// }

impl<F: Field> AddAssign for AffinePoint<F>
where
    F: Neg<Output = F>,
    for<'a> &'a F: Add<&'a F, Output = F>,
    for<'a> &'a F: Sub<&'a F, Output = F>,
{
    fn add_assign(&mut self, rhs: Self) {
        *self = self.add_impl(&rhs);
    }
}

impl<F: Field> AddAssign<&AffinePoint<F>> for AffinePoint<F>
where
    F: Neg<Output = F>,
    for<'a> &'a F: Add<&'a F, Output = F>,
    for<'a> &'a F: Sub<&'a F, Output = F>,
{
    fn add_assign(&mut self, rhs: &AffinePoint<F>) {
        *self = self.add_impl(rhs);
    }
}

impl<F: Field> Sub for AffinePoint<F>
where
    F: Neg<Output = F>,
    for<'a> &'a F: Add<&'a F, Output = F>,
    for<'a> &'a F: Sub<&'a F, Output = F>,
{
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        self.add_impl(&rhs.clone().neg())
    }
}

// impl<F: Field> Sub for &AffinePoint<F>
// where
//     F: Neg<Output = F>
//         + Add<Output = F>
//         + Sub<Output = F>
//         + for<'a> Add<&'a F, Output = F>
//         + for<'a> Sub<&'a F, Output = F>,
//     for<'a> &'a F: Add<&'a F, Output = F>,
//     for<'a> &'a F: Sub<&'a F, Output = F>,
// {
//     type Output = AffinePoint<F>;

//     fn sub(self, rhs: Self) -> Self::Output {
//         self.add_impl(&rhs.clone().neg())
//     }
// }

// impl<F: Field> Sub<&AffinePoint<F>> for AffinePoint<F>
// where
//     F: Neg<Output = F>,
//     for<'c> &'c F: Add<&'c F, Output = F>,
//     for<'c> &'c F: Sub<&'c F, Output = F>,
// {
//     type Output = Self;

//     fn sub(self, rhs: &AffinePoint<F>) -> Self {
//         self.add_impl(&rhs.clone().neg())
//     }
// }

impl<'a, F: Field> Sub for &'a AffinePoint<F>
where
    F: Neg<Output = F>,
    for<'b> &'b F: Add<&'b F, Output = F>,
    for<'b> &'b F: Sub<&'b F, Output = F>,
{
    type Output = AffinePoint<F>;

    fn sub(self, rhs: &'a AffinePoint<F>) -> Self::Output {
        self.add_impl(&rhs.clone().neg())
    }
}

// impl<F: Field> Sub<&Self> for AffinePoint<F>
// where
//     F: Neg<Output = F>
//         + Add<Output = F>
//         + Sub<Output = F>
//         + for<'a> Add<&'a F, Output = F>
//         + for<'a> Sub<&'a F, Output = F>,, //     + for<'a> AddAssign<&'a F>
//                                           //     + for<'a> SubAssign<&'a F>,
//                                           // for<'a> &'a F: Sub<&'a F, Output = F>,
// {
//     type Output = Self;

//     fn sub(self, rhs: &Self) -> Self {
//         // self.add_impl(&rhs.clone().neg())
//         panic!("not implemented");
//     }
// }

// impl<F: Field> Sub<&AffinePoint<F>> for &AffinePoint<F>
// where
//     F: Neg<Output = F>
//         + Add<Output = F>
//         + Sub<Output = F>
//         + for<'a> Add<&'a F, Output = F>
//         + for<'a> Sub<&'a F, Output = F>,
//         + for<'a> AddAssign<&'a F>
//         + for<'a> SubAssign<&'a F>,
//     // for<'a> &'a F: Sub<&'a F, Output = F>,
// {
//     type Output = AffinePoint<F>;

//     fn sub(self, rhs: &AffinePoint<F>) -> Self::Output {
//         panic!("not implemented");
//         // self.add_impl(&rhs.clone().neg())
//     }
// }

impl<F: Field> SubAssign for AffinePoint<F>
where
    F: Neg<Output = F>,
    for<'a> &'a F: Add<&'a F, Output = F>,
    for<'a> &'a F: Sub<&'a F, Output = F>,
{
    fn sub_assign(&mut self, rhs: Self) {
        *self = self.add_impl(&rhs.clone().neg());
    }
}

impl<F: Field> SubAssign<&AffinePoint<F>> for AffinePoint<F>
where
    F: Neg<Output = F>,
    for<'a> &'a F: Add<&'a F, Output = F>,
    for<'a> &'a F: Sub<&'a F, Output = F>,
{
    fn sub_assign(&mut self, rhs: &AffinePoint<F>) {
        *self = self.add_impl(&rhs.clone().neg());
    }
}
