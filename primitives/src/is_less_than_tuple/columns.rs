use afs_derive::AlignedBorrow;

use crate::{is_equal_vec::columns::IsEqualVecAuxCols, is_less_than::columns::IsLessThanAuxCols};

use super::IsLessThanTupleAir;

#[derive(Default, Debug, AlignedBorrow)]
pub struct IsLessThanTupleIoCols<T> {
    pub x: Vec<T>,
    pub y: Vec<T>,
    pub tuple_less_than: T,
}

impl<T: Clone> IsLessThanTupleIoCols<T> {
    pub fn from_slice(slc: &[T], tuple_len: usize) -> Self {
        Self {
            x: slc[0..tuple_len].to_vec(),
            y: slc[tuple_len..2 * tuple_len].to_vec(),
            tuple_less_than: slc[2 * tuple_len].clone(),
        }
    }

    pub fn flatten(&self, buf: &mut [T], start: usize) -> usize {
        buf[start..start + self.x.len()].clone_from_slice(&self.x);
        buf[start + self.x.len()..start + self.x.len() + self.y.len()].clone_from_slice(&self.y);
        buf[start + self.x.len() + self.y.len()] = self.tuple_less_than.clone();
        self.x.len() + self.y.len() + 1
    }

    pub fn width(tuple_len: usize) -> usize {
        tuple_len + tuple_len + 1
    }
}

#[derive(Debug, Clone)]
pub struct IsLessThanTupleAuxCols<T> {
    pub less_than: Vec<T>,
    pub less_than_aux: Vec<IsLessThanAuxCols<T>>,
    pub is_equal_vec_aux: IsEqualVecAuxCols<T>,
    pub less_than_cumulative: Vec<T>,
}

impl<T: Clone> IsLessThanTupleAuxCols<T> {
    pub fn from_slice(slc: &[T], lt_chip: &IsLessThanTupleAir) -> Self {
        let tuple_len = lt_chip.tuple_len();

        let mut iter = slc.iter().cloned();
        let mut take = |n: usize| iter.by_ref().take(n).collect::<Vec<T>>();

        let less_than = take(tuple_len);

        let mut less_than_aux: Vec<IsLessThanAuxCols<T>> = vec![];
        for air in lt_chip.is_less_than_airs.iter() {
            let cur_width = IsLessThanAuxCols::<T>::width(air);
            let less_than_col = IsLessThanAuxCols::from_slice(&take(cur_width));
            less_than_aux.push(less_than_col);
        }

        let prods = take(tuple_len);
        let invs = take(tuple_len);
        let is_equal_vec_aux = IsEqualVecAuxCols { prods, invs };

        let less_than_cumulative = take(tuple_len);

        Self {
            less_than,
            less_than_aux,
            is_equal_vec_aux,
            less_than_cumulative,
        }
    }

    pub fn flatten(&self, buf: &mut [T], start: usize) -> usize {
        let mut cum_len = 0;
        buf[start..start + self.less_than.len()].clone_from_slice(&self.less_than);
        cum_len += self.less_than.len();
        for i in 0..self.less_than_aux.len() {
            let aux_width = self.less_than_aux[i].flatten(buf, start + cum_len);
            cum_len += aux_width;
        }
        buf[start + cum_len..start + cum_len + self.is_equal_vec_aux.prods.len()]
            .clone_from_slice(&self.is_equal_vec_aux.prods);
        cum_len += self.is_equal_vec_aux.prods.len();
        buf[start + cum_len..start + cum_len + self.is_equal_vec_aux.invs.len()]
            .clone_from_slice(&self.is_equal_vec_aux.invs);
        cum_len += self.is_equal_vec_aux.invs.len();
        buf[start + cum_len..start + cum_len + self.less_than_cumulative.len()]
            .clone_from_slice(&self.less_than_cumulative);
        cum_len += self.less_than_cumulative.len();

        cum_len
    }

    pub fn width(lt_air: &IsLessThanTupleAir) -> usize {
        let mut width = 2 * lt_air.tuple_len();
        for air in lt_air.is_less_than_airs.iter() {
            width += IsLessThanAuxCols::<T>::width(air);
        }
        // TODO: the +1 here is a hack to account for the specific way IsEqualVec chip
        // is used in this chip. We should use IsEqualVec as a SubAir (instead of duplicating
        // the logic of trace generation, AIR constraints etc) and clean this up
        width += IsEqualVecAuxCols::<T>::width(lt_air.tuple_len()) + 1;

        width
    }
}

#[derive(Debug)]
pub struct IsLessThanTupleCols<T> {
    pub io: IsLessThanTupleIoCols<T>,
    pub aux: IsLessThanTupleAuxCols<T>,
}

impl<T: Clone> IsLessThanTupleCols<T> {
    pub fn from_slice(slc: &[T], lt_air: &IsLessThanTupleAir) -> Self {
        let tuple_len = lt_air.tuple_len();

        let io = IsLessThanTupleIoCols::from_slice(&slc[..2 * tuple_len + 1], tuple_len);
        let aux = IsLessThanTupleAuxCols::from_slice(&slc[2 * tuple_len + 1..], lt_air);

        Self { io, aux }
    }

    pub fn flatten(&self, buf: &mut [T], start: usize) -> usize {
        let io_len = self.io.flatten(buf, start);
        let aux_len = self.aux.flatten(buf, start + io_len);
        io_len + aux_len
    }

    pub fn width(lt_air: &IsLessThanTupleAir) -> usize {
        IsLessThanTupleIoCols::<T>::width(lt_air.tuple_len())
            + IsLessThanTupleAuxCols::<T>::width(lt_air)
    }
}
