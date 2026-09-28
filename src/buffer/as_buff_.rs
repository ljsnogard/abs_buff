use core::{
    borrow::{Borrow, BorrowMut},
    mem::MaybeUninit,
};

use super::buff_::{TrMaybeUninitSlice, TrMaybeUninitSliceMut};

pub trait TrAsBuffer<T> {
    /// Explicitly declare that the termination of evaluation for
    /// `TrMaybeUninit` be `core::mem::MaybeUninit`.
    fn as_buff(&self) -> &[MaybeUninit<T>];
}

pub trait TrAsBufferMut<T>
where
    Self: TrAsBuffer<T>,
{
    fn as_mut_buff(&mut self) -> &mut [MaybeUninit<T>];
}

//-- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ----
// impl TrBuffer for `&<[MaybeUninit<T>]>`
//-- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ----

impl<C, T> TrAsBuffer<T> for C
where
    C: Borrow<[T]>,
{
    #[inline]
    default fn as_buff(&self) -> &[MaybeUninit<T>] {
        let len = self.borrow().len();
        let data = self.borrow().as_ptr() as *const MaybeUninit<T>;
        unsafe { core::slice::from_raw_parts(data, len) }
    }
}

impl<T> TrAsBuffer<T> for [T]
where
    T: Copy,
{
    fn as_buff(&self) -> &[MaybeUninit<T>] {
        self.as_uninit_slice()
    }
}

//-- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ----
// impl TrBuffer TrBufferMut for `&mut [MaybeUninit<T>]`
//-- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ----

impl<C, T> TrAsBufferMut<T> for C
where
    C: BorrowMut<[T]>,
{
    #[inline]
    default fn as_mut_buff(&mut self) -> &mut [MaybeUninit<T>] {
        let len = self.borrow_mut().len();
        let data = self.borrow_mut().as_ptr() as *mut MaybeUninit<T>;
        unsafe { core::slice::from_raw_parts_mut(data, len) }
    }
}

impl<T> TrAsBufferMut<T> for [T]
where
    T: Copy,
{
    fn as_mut_buff(&mut self) -> &mut [MaybeUninit<T>] {
        self.as_uninit_slice_mut()
    }
}
