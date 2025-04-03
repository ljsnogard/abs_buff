use core::error::Error;

use abs_sync::cancellation::TrMayCancel;

use anylr::SomeOf;

use crate::{BuffWriteAsOutput, Demand, TrBuffSegmMut, TrOutput};

/// Buffer that will emit zero or more segments for producer (and update cursor)
pub trait TrBuffWrite<T = u8> {
    type WriterSegm<'a>: 'a + TrBuffSegmMut<T>
    where
        Self: 'a;

    type Segments<'a>: 'a + IntoIterator<Item = Self::WriterSegm<'a>>
    where
        Self: 'a;

    type WriteAsync<'a>: TrMayCancel<'a,
        MayCancelOutput = SomeOf<Self::Segments<'a>, Self::Err>>
    where
        Self: 'a;

    type Err: Error;

    /// Lend some segments for writing in an async manner. The total amount of
    /// items is specified by the parameter `demand`.
    fn write_async(
        &mut self,
        demand: &Demand<usize>,
    ) -> Self::WriteAsync<'_>;

    fn as_output(&mut self) -> impl TrOutput<T>
    where
        Self: Sized,
    {
        BuffWriteAsOutput::<&mut Self, Self, T>::new(self)
    }
}

pub trait TrBuffTryWrite<T = u8>: TrBuffWrite<T> {
    fn try_write(
        &mut self,
        demand: &Demand<usize>,
    ) -> SomeOf<Self::Segments<'_>, Self::Err>;
}
