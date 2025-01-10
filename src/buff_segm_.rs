use core::{
    borrow::{Borrow, BorrowMut},
    cmp,
    mem::{self, MaybeUninit},
    ptr,
};

pub trait TrBuffSegmView
where
    Self: AsRef<[Self::Item]> + Borrow<[Self::Item]>,
{
    type Item: Sized;

    fn is_empty(&self) -> bool {
        self.as_ref().is_empty()
    }

    fn len(&self) -> usize {
        self.as_ref().len()
    }
}

pub trait TrBuffSegmRef<'a, T>
where
    Self: TrBuffSegmView<Item = T>,
{
    /// Take a sliced segment with a length limited by the given length out
    /// from this segment, reducing the length of this segment.
    fn take_segm_ref(
        &mut self,
        length: usize,
    ) -> impl TrBuffSegmRef<'_, T>;
}

pub trait TrBuffSegmMut<'a, T>
where
    Self: TrBuffSegmView<Item = MaybeUninit<T>> +
        AsMut<[MaybeUninit<T>]> + 
        BorrowMut<[MaybeUninit<T>]>,
{
    /// Move items in source into this segment, reducing the length of both the
    /// source and the target segment (this segment).
    fn dump_from_segm<'s, S>(&'s mut self, source: &'s mut S) -> usize
    where
        S: TrBuffSegmRef<'s, T>,
    {
        let mut dst = self.take_segm_mut(source.len());
        let src = source.take_segm_ref(dst.len());
        let count = cmp::min(dst.len(), src.len());
        if count == 0 {
            return count;
        }
        let dst: &mut [MaybeUninit<T>] = dst.borrow_mut();
        let dst = &mut dst[0] as *mut MaybeUninit<T> as *mut T;
        let src : &[T] = src.borrow();
        let src = &src[0] as *const T;
        unsafe { ptr::copy_nonoverlapping(src, dst, count) };
        count
    }

    /// Clone items in source into this segment, reducing the length of this 
    /// segment.
    fn clone_from_slice(&mut self, source: &[T]) -> usize
    where
        T: Clone,
    {
        let mut dst = self.take_segm_mut(source.len());
        let count = dst.len();
        if count == 0 {
            return count;
        }
        let dst: &mut [MaybeUninit<T>] = dst.borrow_mut();
        let src = &source[..count];
        if mem::needs_drop::<T>() {
            for i in 0..count {
                let m = &mut dst[i];
                m.write(src[i].clone());
            }
        } else {
            let dst = unsafe {
                let p = dst as *mut _ as *mut [T];
                &mut *p
            };
            dst.clone_from_slice(src);
        }
        count
    }

    /// Take a sliced segment with a length limited by the given length out
    /// from this segment, reducing the length of this segment.
    fn take_segm_mut(
        &mut self, 
        length: usize,
    ) -> impl TrBuffSegmMut<'_, T>;
}
