use core::{
    error::Error,
    iter::IntoIterator,
    mem::MaybeUninit,
    ops::DerefMut,
};

use abs_sync::cancellation::TrIntoFutureMayCancel;

/// Buffer that will lend zero or more slices for writing (and update cursor)
pub trait TrBuffIterWrite<T = u8> {
    type SliceMut<'a>: DerefMut<Target = [MaybeUninit<T>]> +
        IntoIterator<Item = MaybeUninit<T>>
    where
        Self: 'a;

    type BuffIter<'a>: IntoIterator<Item = Self::SliceMut<'a>>
    where
        Self: 'a;

    type WriteAsync<'a>: TrIntoFutureMayCancel<'a,
        MayCancelOutput = Result<Self::BuffIter<'a>, Self::Err>>
    where
        Self: 'a;

    type Err: Error;

    /// Lend some slices for writing. The total length of these slices will be
    /// no greater than the length given in the argument.
    fn write_async(&mut self, length: usize) -> Self::WriteAsync<'_>;
}

pub trait TrBuffIterTryWrite<T = u8>: TrBuffIterWrite<T> {
    fn try_write(
        &mut self,
        length: usize,
    ) -> Result<Self::BuffIter<'_>, Self::Err>;
}
