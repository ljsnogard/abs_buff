use abs_sync::cancellation::TrIntoFutureMayCancel;

pub trait TrIoAbortReport {
    type LastErr;

    fn perform_len(&self) -> usize;

    fn last_error(&self) -> &Self::LastErr;
}

pub trait TrChunkFiller<T: Clone = u8> {
    type FillError: TrIoAbortReport;

    type FillAsync<'a>: TrIntoFutureMayCancel<'a, MayCancelOutput =
        Result<usize, Self::FillError>>
    where
        T: 'a,
        Self: 'a;

    fn can_fill(&mut self) -> bool;

    fn fill_async<'a>(
        &'a mut self,
        target: &'a mut [T],
    ) -> Self::FillAsync<'a>;
}

pub trait TrChunkWriter<T: Clone = u8> {
    type WriterError: TrIoAbortReport;

    type WriteAsync<'a>: TrIntoFutureMayCancel<'a, MayCancelOutput = 
        Result<usize, Self::WriterError>>
    where
        T: 'a,
        Self: 'a;

    fn can_write(&mut self) -> bool;

    fn write_async<'a>(
        &'a mut self,
        source: &'a [T],
    ) -> Self::WriteAsync<'a>;
}

#[derive(Debug)]
pub struct IoAbortReport<E> {
    perform_len_: usize,
    last_error_: E,
}

impl<E> IoAbortReport<E> {
    pub const fn new(perform_len: usize, last_error: E) -> Self {
        IoAbortReport {
            perform_len_: perform_len,
            last_error_: last_error,
        }
    }

    pub const fn perform_len(&self) -> usize {
        self.perform_len_
    }

    pub const fn last_error(&self) -> &E {
        &self.last_error_
    }
}

impl<E> TrIoAbortReport for IoAbortReport<E> {
    type LastErr = E;

    #[inline(always)]
    fn perform_len(&self) -> usize {
        IoAbortReport::perform_len(self)
    }

    #[inline(always)]
    fn last_error(&self) -> &E {
        IoAbortReport::last_error(self)
    }
}
