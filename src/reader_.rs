use core::{
    error::Error,
    iter::IntoIterator,
    ops::Deref,
};

use abs_sync::cancellation::TrIntoFutureMayCancel;

/// Buffer that will lend zero or more slices for reading (and update cursor)
pub trait TrBuffIterRead<T = u8> {
    type SliceRef<'a>: Deref<Target = [T]> + IntoIterator<Item = T>
    where
        Self: 'a;

    type BuffIter<'a>: IntoIterator<Item = Self::SliceRef<'a>>
    where
        Self: 'a;

    type ReadAsync<'a>: TrIntoFutureMayCancel<'a,
        MayCancelOutput = Result<Self::BuffIter<'a>, Self::Err>>
    where
        Self: 'a;

    type Err: Error;

    /// Lend some slices for reading. The total length of these slices will be
    /// no greater than the length given in the argument.
    fn read_async(&mut self, length: usize) -> Self::ReadAsync<'_>;
}

pub trait TrBuffIterTryRead<T = u8>: TrBuffIterRead<T> {
    fn try_read(
        &mut self,
        length: usize,
    ) -> Result<Self::BuffIter<'_>, Self::Err>;
}
