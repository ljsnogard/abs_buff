use abs_cancel::TrMayCancel;
use anylr::SomeOf;

use crate::{
    Demand,
    buffer::TrBuffSegmMut,
    error::{TrTaggedError, WriteErrTag},
};

pub trait TrBuffTryWrite<T = u8> {
    type SegmMut<'f>: TrBuffSegmMut<'f, T> where Self: 'f;

    type Err: TrTaggedError<WriteErrTag>;

    fn try_write<'f>(
        &'f mut self,
        demand: &'f Demand<usize>,
    ) -> SomeOf<Self::SegmMut<'f>, Self::Err>;

    fn try_write_one(&mut self, x: T) -> bool {
        let demand = Demand::at_least(1);
        let Option::Some(mut segm) = self.try_write(&demand).pick_left() else {
            return false;
        };
        let buff = [x];
        segm.move_items_from_as_buff(&buff) == 1
    }
}

/// A kind of buffer that owns the memory for writing data by lending some
/// segments to the producer.
///
/// This design is to keep compatible with `io_uring` and polling model.
pub trait TrBuffWrite<T = u8>
where
    Self: TrBuffTryWrite<T>,
{
    type WriteAsync<'f>: TrMayCancel<'f, MayCancelOutput =
        SomeOf<Self::SegmMut<'f>, Self::Err>>
    where
        Self: 'f;

    /// Lend some segments for writing in an async manner. The total amount of
    /// items is specified by the parameter `demand`.
    fn write_async<'f>(
        &'f mut self,
        demand: &'f Demand<usize>,
    ) -> Self::WriteAsync<'f>;
}
