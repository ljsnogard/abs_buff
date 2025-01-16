use core::{
    error::Error,
    iter::IntoIterator,
};

use abs_sync::cancellation::TrMayCancel;

use crate::TrBuffSegmMut;

/// Buffer that will emit zero or more segments for producer (and update cursor)
pub trait TrBuffIterWrite<T = u8> {
    type SegmMut<'a>: 'a + TrBuffSegmMut<T>
    where
        Self: 'a;

    /// Zero or more segments returns to the caller of
    /// [write_async](TrBuffIterWrite::write_async)
    type Segments<'a>: IntoIterator<Item = Self::SegmMut<'a>>
    where
        Self: 'a;

    type WriteAsync<'a>: TrMayCancel<'a,
        MayCancelOutput = Result<Self::Segments<'a>, Self::Err>>
    where
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
    ) -> Result<Self::Segments<'_>, Self::Err>;
}
