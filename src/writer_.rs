use core::{
    borrow::BorrowMut,
    error::Error,
    iter::IntoIterator,
    mem::MaybeUninit,
};

use abs_sync::cancellation::TrIntoFutureMayCancel;

/// Buffer that will lend zero or more slices for writing (and update cursor)
pub trait TrBuffIterWrite<T = u8> {
    type SegmMut<'a>: BorrowMut<[MaybeUninit<T>]>
    where
        T: 'a,
        Self: 'a;

    /// Zero or more segments returns to the caller of
    /// [write_async](TrBuffIterWrite::write_async)
    type SegmIter<'a>: IntoIterator<Item = Self::SegmMut<'a>>
    where
        T: 'a,
        Self: 'a;

    type WriteAsync<'a>: TrIntoFutureMayCancel<'a,
        MayCancelOutput = Result<Self::SegmIter<'a>, Self::Err>>
    where
        T: 'a,
        Self: 'a;

    type Err: Error;

    /// Lend some segments for writing. The total length of these segments will
    /// be no greater than the length given in the argument.
    fn write_async(&mut self, length: usize) -> Self::WriteAsync<'_>;
}

pub trait TrBuffIterTryWrite<T = u8>: TrBuffIterWrite<T> {
    fn try_write(
        &mut self,
        length: usize,
    ) -> Result<Self::SegmIter<'_>, Self::Err>;
}
