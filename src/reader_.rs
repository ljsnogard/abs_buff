﻿use core::{iter::IntoIterator, ops::Deref};

use abs_sync::cancellation::TrIntoFutureMayCancel;

/// Buffer that will lend zero or more slices for reading (and update cursor)
pub trait TrBuffIterRead<T = u8>
where
    T: Clone,
{
    type SliceRef<'a>: Deref<Target = [T]> where Self: 'a;
    type BuffIter<'a>: IntoIterator<Item = Self::SliceRef<'a>> where Self: 'a;
    type Err: Error;

    type ReadAsync<'a>: TrIntoFutureMayCancel<'a, MayCancelOutput =
        Result<Self::BuffIter<'a>, Self::Err>>
    where
        Self: 'a;

    /// Lend some slices with given max length for reading.
    fn read_async(&mut self, length: usize) -> Self::ReadAsync<'_>;
}
