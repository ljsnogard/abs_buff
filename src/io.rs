use core::{
    error::Error,
    mem::MaybeUninit, ptr,
};

use abs_sync::cancellation::TrMayCancel;

use anylr::SomeOf;

/// Unbuffered input device
pub trait TrUnbufferedInput<T = u8> {
    type Err : Error;

    type ReadAsync<'a>:
        TrMayCancel<'a, MayCancelOutput = SomeOf<usize, Self::Err>>
    where
        T: 'a,
        Self: 'a;

    /// Move the data out of the device and into the specified target buffer.
    fn read_async<'a>(
        &'a mut self, target:
        &'a mut [MaybeUninit<T>],
    ) -> Self::ReadAsync<'a>;
}

/// Unbuffered output device
pub trait TrUnbufferedOutput<T = u8> {
    type Err : Error;

    type WriteAsync<'a>:
        TrMayCancel<'a, MayCancelOutput = SomeOf<usize, Self::Err>>
    where
        T: 'a,
        Self: 'a;

    /// Move data from the specified source into this output device
    fn write_async<'a>(
        &'a mut self,
        source: &'a [MaybeUninit<T>],
    ) -> Self::WriteAsync<'a>;

    /// Clone data from the specified source buffer into this output device 
    fn write_cloned_async<'a>(
        &'a mut self,
        source: &'a [T],
    ) -> Self::WriteAsync<'a>
    where
        T: Clone,
    {
        unsafe {
            let src_head = &source[0] as *const T as *const MaybeUninit<T>;
            let slice = ptr::slice_from_raw_parts(src_head, source.len());
            self.write_async(&*slice)
        }
    }
}
