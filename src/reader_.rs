use core::error::Error;

use abs_sync::cancellation::TrMayCancel;

use anylr::SomeOf;

use crate::{BuffReadAsInput, Demand, TrInput, TrBuffSegmRef};

/// Buffer that will emit zero or more segments for consumer (and update cursor)
pub trait TrBuffRead<T = u8> {
    type ReaderSegm<'a>: 'a + TrBuffSegmRef<T>
    where
        Self: 'a;

    type Segments<'a>: 'a + IntoIterator<Item = Self::ReaderSegm<'a>>
    where
        Self: 'a;

    type ReadAsync<'a>: TrMayCancel<'a,
        MayCancelOutput = SomeOf<Self::Segments<'a>, Self::Err>>
    where
        Self: 'a;

    type Err: Error;

    /// Lend some segments for reading in async manner. The amount of items
    /// is specified by the parameter `demand`.
    fn read_async(
        &mut self,
        demand: &Demand<usize>,
    ) -> Self::ReadAsync<'_>;

    fn as_input(&mut self) -> impl TrInput<T>
    where
        Self: Sized,
    {
        BuffReadAsInput::<&mut Self, Self, T>::new(self)
    }
}

pub trait TrBuffTryRead<T = u8>: TrBuffRead<T> {
    fn try_read(
        &mut self,
        demand: &Demand<usize>,
    ) -> SomeOf<Self::Segments<'_>, Self::Err>;
}
