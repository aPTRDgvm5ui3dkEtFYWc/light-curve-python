use conv::*;
use itertools::Itertools;
use ndarray::{Array1, Array2, ScalarOperand};
use std::io::Write;

mod erf;
pub use erf::{ErfEps1Over1e3Float, ErrorFunction, LibMFloat};

mod float_trait;
pub use float_trait::Float;

pub trait Normalisable:
    ApproxInto<u8, DefaultApprox>
    + num_traits::Num
    + num_traits::NumOps
    + PartialOrd
    + ScalarOperand
    + Copy
{
    fn clamp(self, min: Self, max: Self) -> Self;
    fn max_u8() -> Self;
}

impl Normalisable for usize {
    fn clamp(self, min: Self, max: Self) -> Self {
        match self {
            _ if self < min => min,
            x if self <= max => x,
            _ => max,
        }
    }

    #[inline]
    fn max_u8() -> Self {
        255
    }
}

impl Normalisable for f32 {
    fn clamp(self, min: Self, max: Self) -> Self {
        self.clamp(min, max)
    }

    #[inline]
    fn max_u8() -> Self {
        255.0
    }
}

impl Normalisable for f64 {
    fn clamp(self, min: Self, max: Self) -> Self {
        self.clamp(min, max)
    }

    #[inline]
    fn max_u8() -> Self {
        255.0
    }
}

pub struct Grid<T> {
    start: T, // coordinate of the left border of the leftmost cell
    end: T,   // coordinate of the right border of the rightmost cell
    n: usize,
    // length: T, // distance from the left border of the leftmost cell to the right border of the rightmost cell
    cell_size: T,
    borders: Array1<T>,
}

#[allow(clippy::len_without_is_empty)]
impl<T> Grid<T>
where
    T: Float,
{
    pub fn new(start: T, end: T, n: usize) -> Self {
        assert!(end > start);
        let cell_size = (end - start) / n.value_as::<T>().unwrap();
        let borders = Array1::linspace(start, end, n + 1);
        Self {
            start,
            end,
            n,
            // length,
            cell_size,
            borders,
        }
    }

    pub fn len(&self) -> usize {
        self.n
    }

    fn idx(&self, x: T) -> CellIndex {
        match x {
            _ if x < self.start => CellIndex::LowerMin,
            _ if x >= self.end => CellIndex::GreaterMax,
            _ => CellIndex::Value(((x - self.start) / self.cell_size).approx_into().unwrap()),
        }
    }
}

enum CellIndex {
    LowerMin,
    GreaterMax,
    Value(usize),
}

pub struct DmDt<T> {
    pub lgdt_grid: Grid<T>,
    pub dm_grid: Grid<T>,
}

impl<T> DmDt<T>
where
    T: Float,
{
    /// N lg_dt by N dm
    pub fn shape(&self) -> (usize, usize) {
        (self.lgdt_grid.n, self.dm_grid.n)
    }

    pub fn convert_lc_to_points(&self, t: &[T], m: &[T]) -> Array2<usize> {
        let mut a = Array2::zeros((self.lgdt_grid.n, self.dm_grid.n));
        for (i1, (&x1, &y1)) in t.iter().zip(m.iter()).enumerate() {
            for (&x2, &y2) in t[i1 + 1..].iter().zip(m[i1 + 1..].iter()) {
                let lgdt = T::log10(x2 - x1);
                let idx_lgdt = match self.lgdt_grid.idx(lgdt) {
                    CellIndex::LowerMin => continue,
                    CellIndex::GreaterMax => break,
                    CellIndex::Value(idx_lgdt) => idx_lgdt,
                };
                let dm = y2 - y1;
                let idx_dm = match self.dm_grid.idx(dm) {
                    CellIndex::Value(idx_dm) => idx_dm,
                    CellIndex::LowerMin | CellIndex::GreaterMax => continue,
                };
                a[(idx_lgdt, idx_dm)] += 1;
            }
        }
        a
    }

    pub fn convert_lc_to_gausses(&self, t: &[T], m: &[T], w: &[T], erf: &ErrorFunction) -> Array2<T>
    where
        T: LibMFloat + ErfEps1Over1e3Float,
    {
        let mut a = Array2::zeros((self.lgdt_grid.n, self.dm_grid.n));
        for (i1, ((&x1, &y1), &dm_w1)) in t.iter().zip(m.iter()).zip(w.iter()).enumerate() {
            for ((&x2, &y2), &dm_w2) in t[i1 + 1..]
                .iter()
                .zip(m[i1 + 1..].iter())
                .zip(w[i1 + 1..].iter())
            {
                let lgdt = T::log10(x2 - x1);
                let idx_lgdt = match self.lgdt_grid.idx(lgdt) {
                    CellIndex::LowerMin => continue,
                    CellIndex::GreaterMax => break,
                    CellIndex::Value(idx_lgdt) => idx_lgdt,
                };
                let dm = y2 - y1;
                let dm_w = dm_w1 + dm_w2;
                a.row_mut(idx_lgdt)
                    .iter_mut()
                    .zip(
                        self.dm_grid
                            .borders
                            .iter()
                            .map(|&dm_border| erf.normal_cdf(dm_border, dm, dm_w))
                            .tuple_windows()
                            .take_while(|(a, _b)| !a.is_one())
                            .map(|(a, b)| b - a),
                    )
                    .for_each(|(cell, value)| *cell += value);
            }
        }
        a
    }
}

pub fn normalise<T>(a: &Array2<T>) -> Array2<u8>
where
    T: Normalisable + std::fmt::Debug,
{
    let max = *a.iter().max_by(|&x, &y| x.partial_cmp(y).unwrap()).unwrap();
    if max.is_zero() {
        Array2::zeros((a.nrows(), a.ncols()))
    } else {
        let normalised = a * T::max_u8() / max;
        normalised.mapv(|x| x.clamp(T::zero(), T::max_u8()).approx_into().unwrap())
    }
}

pub fn to_png<W>(w: W, a: &Array2<u8>) -> Result<(), png::EncodingError>
where
    W: Write,
{
    let transposed = {
        let mut b = Array2::zeros((a.ncols(), a.nrows()));
        b.assign(&a.t());
        b
    };
    let mut encoder = png::Encoder::new(w, transposed.ncols() as u32, transposed.nrows() as u32);
    encoder.set_color(png::ColorType::Grayscale);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(transposed.as_slice_memory_order().unwrap())?;
    Ok(())
}
