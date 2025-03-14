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

    /// Returns if the elements are all consumed, or never existing.
    fn is_empty(&self) -> bool {
        self.as_ref().is_empty()
    }

    /// The length of the unconsumed part of the segment.
    fn len(&self) -> usize {
        self.as_ref().len()
    }

    /// Iterate over the elements of the internal buffer retained by the segment
    /// and retrieve as pointers.
    fn iter_ptr(&self) -> impl Iterator<Item = *const Self::Item>;

    /// Returns the length of the borrowed segment, no matter the elements are
    /// consumed or not. This is usually used by the reclaim function.
    fn borrowed_len(&self) -> usize;
}

pub trait TrBuffSegmRef<T>
where
    Self: TrBuffSegmView<Item = T>,
{
    /// Take a sliced segment out from this segment with a length limited by
    /// by the argument, reducing the length of this segment when the taken
    /// slice drops.
    fn take_segm_ref(
        &mut self,
        length: usize,
    ) -> impl TrBuffSegmRef<T>;

    fn fill_into_buff(&mut self, target: &mut [MaybeUninit<T>]) -> usize {
        let count = cmp::min(target.len(), self.len());
        let src = self.take_segm_ref(count);
        let src_head = (&src.as_ref()[0]) as *const T;
        let dst_head = (&mut target[0]) as *mut MaybeUninit<T> as *mut T;

        // This is sound because it is semantically a move operation since `src`
        // will drop and convert the "copied" items into `MaybeUninit`
        unsafe { ptr::copy_nonoverlapping(src_head, dst_head, count) };
        count
    }
}

pub trait TrBuffSegmMut<T>
where
    Self: TrBuffSegmView<Item = MaybeUninit<T>> +
        AsMut<[MaybeUninit<T>]> + 
        BorrowMut<[MaybeUninit<T>]>,
{
    /// Take a sliced segment out from this segment with a length limited by
    /// by the argument, reducing the length of this segment when the taken
    /// slice drops.
    fn take_segm_mut(
        &mut self, 
        length: usize,
    ) -> impl TrBuffSegmMut<T>;

    /// Move items in source into this segment, reducing the length of both the
    /// source and the target segment (this segment).
    fn dump_from_segm<S>(&mut self, source: &mut S) -> usize
    where
        S: TrBuffSegmRef<T>,
    {
        let count = cmp::min(source.len(), self.len());
        if count == 0 {
            return count;
        }
        let src = source.take_segm_ref(count);
        let src = src.as_ref();

        // This is souned because the source segment is not expected to drop
        // the element items when it drops. Thus this is semantically a move.
        let src = unsafe {
            let head = &src[0] as *const T as *const MaybeUninit<T>;
            let p = ptr::slice_from_raw_parts(head, src.len());
            &*p
        };
        self.dump_from_slice(src)
    }

    fn dump_from_slice(&mut self, source: &[MaybeUninit<T>]) -> usize {
        let count = cmp::min(source.len(), self.len());
        if count == 0 {
            return count;
        }
        let mut dst = self.take_segm_mut(count);
        let dst = (&mut dst.as_mut()[0]) as *mut MaybeUninit<T> as *mut T;
        let src = &source[0..count];
        let src = &src[0] as *const MaybeUninit<T> as *const T;
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
}
