use core::{
    error::Error,
    iter::IntoIterator,
};

use abs_sync::cancellation::TrMayCancel;

use anylr::SomeOf;

use crate::{
    io::TrUnbufferedInput,
    TrBuffSegmRef,
};

/// Buffer that will emit zero or more segments for consumer (and update cursor)
pub trait TrBuffIterRead<T = u8> {
    type SegmRef<'a>: 'a + TrBuffSegmRef<T>
    where
        Self: 'a;

    type Segments<'a>: IntoIterator<Item = Self::SegmRef<'a>>
    where
        Self: 'a;

    type ReadAsync<'a>: TrMayCancel<'a,
        MayCancelOutput = SomeOf<Self::Segments<'a>, Self::Err>>
    where
        Self: 'a;

    type Err: Error;

    /// Borrow some segments for reading. The total length of these segments
    /// will be no greater than the length given in the argument.
    fn read_async(&mut self, length: usize) -> Self::ReadAsync<'_>;

    fn as_input(&mut self) -> impl TrUnbufferedInput<T>
    where
        Self: Sized,
    {
        crate::buff_read_as_input_::BuffReadAsInput::<&mut Self, Self, T>::new(self)
    }
}

pub trait TrBuffIterTryRead<T = u8>: TrBuffIterRead<T> {
    fn try_read(
        &mut self,
        length: usize,
    ) -> SomeOf<Self::Segments<'_>, Self::Err>;
}
