﻿use core::{iter::IntoIterator, ops::DerefMut};

use abs_sync::cancellation::TrIntoFutureMayCancel;

/// Buffer that will lend zero or more slices for writing (and update cursor)
pub trait TrBuffIterWrite<T = u8>
where
    T: Clone,
{
    type SliceMut<'a>: DerefMut<Target = [T]> where Self: 'a;
    type BuffIter<'a>: IntoIterator<Item = Self::SliceMut<'a>> where Self: 'a;
    type Err: Error;

    type WriteAsync<'a>: TrIntoFutureMayCancel<'a, MayCancelOutput =
        Result<Self::BuffIter<'a>, Self::Err>>
    where
        Self: 'a;

    /// Lend some slices with the given max length for writing.
    fn write_async(&mut self, length: usize) -> Self::WriteAsync<'_>;
}
