use core::{
    borrow::BorrowMut,
    marker::PhantomData,
    mem::MaybeUninit,
    pin::Pin,
};

use abs_sync::{
    cancellation::{TrCancellationToken, TrMayCancel},
    gen_mcf_macro::gen_may_cancel_future,
};
use anylr::SomeOf;

use crate::{Demand, TrBuffWrite, TrBuffSegmMut, TrOutput};

pub struct BuffWriteAsOutput<B, W, T>(B, PhantomData<W>, PhantomData<[T]>)
where
    B: BorrowMut<W>,
    W: TrBuffWrite<T>;

impl<B, W, T> BuffWriteAsOutput<B, W, T>
where
    B: BorrowMut<W>,
    W: TrBuffWrite<T>,
{
    pub const fn new(r: B) -> Self {
        BuffWriteAsOutput(r, PhantomData, PhantomData)
    }
}

impl<'a, W, T> From<&'a mut W> for BuffWriteAsOutput<&'a mut W, W, T>
where
    W: TrBuffWrite<T>,
{
    fn from(value: &'a mut W) -> Self {
        BuffWriteAsOutput::<&'a mut W, W, T>::new(value)
    }
}

impl<W, T> From<W> for BuffWriteAsOutput<W, W, T>
where
    W: TrBuffWrite<T>,
{
    fn from(value: W) -> Self {
        BuffWriteAsOutput::new(value)
    }
}

impl<B, W, T> TrOutput<T> for BuffWriteAsOutput<B, W, T>
where
    B: BorrowMut<W>,
    W: TrBuffWrite<T>,
{
    type Err = <W as TrBuffWrite<T>>::Err;

    type WriteAsync<'a> = BuffWriteOutputAsync<'a, W, T>
    where
        T: 'a,
        Self: 'a;

    fn write_async<'a>(
        &'a mut self,
        source: &'a [MaybeUninit<T>],
    ) -> Self::WriteAsync<'a> {
        BuffWriteOutputAsync(self.0.borrow_mut(), source)
    }
}

#[gen_may_cancel_future(BuffWriteOutput)]
async fn buff_write_output_async<'f, W, T, C>(
    writer: &'f mut W,
    source: &'f [MaybeUninit<T>],
    cancel: Pin<&'f mut C>,
) -> SomeOf<usize, <W as TrBuffWrite<T>>::Err>
where 
    W: TrBuffWrite<T>,
    C: TrCancellationToken,
{
    writer
        .write_async(Demand::at_most(source.len()))
        .may_cancel_with(cancel)
        .await
        .map_left(|mut s| s.dump_from_slice(source))
}
