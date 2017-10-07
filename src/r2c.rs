use super::FLAG;
use super::aligned_vec::*;
use super::error::*;
use super::pair::{Pair, ToPair};
use super::plan::R2C;

use ffi;

use ndarray::*;
use num_traits::Zero;
use std::marker::PhantomData;

/// Setting for 1-dimensional R2C transform
#[derive(Debug, Clone, Copy, new)]
pub struct R2C1D {
    n: usize,
    flag: FLAG,
}

/// Utility function to generage 1-dimensional R2C setting
pub fn r2c_1d(n: usize) -> R2C1D {
    R2C1D {
        n,
        flag: ffi::FFTW_MEASURE,
    }
}

impl<R, C> ToPair<R, C> for R2C1D
where
    (R, C): R2C<Real = R, Complex = C>,
    R: AlignedAllocable + Zero,
    C: AlignedAllocable + Zero,
{
    type Dim = Ix1;
    fn to_pair(&self) -> Result<Pair<R, C, Ix1>> {
        let mut field = AlignedVec::<R>::new(self.n);
        let mut coef = AlignedVec::<C>::new(self.n / 2 + 1);
        let forward = unsafe { <(R, C) as R2C>::r2c_1d(self.n, &mut field, &mut coef, self.flag) };
        let backward = unsafe { <(R, C) as R2C>::c2r_1d(self.n, &mut coef, &mut field, self.flag) };
        Pair {
            field: field,
            coef: coef,
            logical_size: self.n,
            forward: forward,
            backward: backward,
            phantom: PhantomData,
        }.null_checked()
    }
}
