use core::{marker::PhantomData, ops::{Deref, DerefMut}};

pub trait TrReclaimIoSliceRef<T: Clone>: Sized {
    fn reclaim_ref(self, slice_ref: &mut IoSliceRef<'_, T, Self>);
}

pub trait TrReclaimIoSliceMut<T: Clone>: Sized {
    fn reclaim_mut(self, slice_mut: &mut IoSliceMut<'_, T, Self>);
}

pub struct NoReclaim<T: Clone>(PhantomData<[T]>);

impl<T: Clone> TrReclaimIoSliceRef<T> for NoReclaim<T> {
    fn reclaim_ref(self, slice_ref: &mut IoSliceRef<'_, T, Self>) {
        let _ = slice_ref;
    }
}

impl<T: Clone> TrReclaimIoSliceMut<T> for NoReclaim<T> {
    fn reclaim_mut(self, slice_mut: &mut IoSliceMut<'_, T, Self>) {
        let _ = slice_mut;
    }
}

pub struct IoSliceRef<'a, T, R>
where
    T: Clone,
    R: TrReclaimIoSliceRef<T>,
{
    slice_ref_: &'a [T],
    reclaimer_: Option<R>,
}

impl<'a, T, R> IoSliceRef<'a, T, R>
where
    T: Clone,
    R: TrReclaimIoSliceRef<T>,
{
    pub fn new_with_reclaimer(
        slice_ref: &'a [T],
        reclaimer: R,
    ) -> Self {
        IoSliceRef {
            slice_ref_: slice_ref,
            reclaimer_: Option::Some(reclaimer),
        }
    }

    /// Creates an instance that will not invoke reclaimer, for example, peeker.
    pub fn new(slice: &'a [T]) -> Self {
        IoSliceRef {
            slice_ref_: slice,
            reclaimer_: Option::None,
        }
    }
}

impl<'a, T, R> Drop for IoSliceRef<'a, T, R>
where
    T: Clone,
    R: TrReclaimIoSliceRef<T>,
{
    fn drop(&mut self) {
        let Option::Some(r) = self.reclaimer_.take() else {
            return;
        };
        r.reclaim_ref(self)
    }
}

impl<'a, T, R> Deref for IoSliceRef<'a, T, R>
where
    T: Clone,
    R: TrReclaimIoSliceRef<T>,
{
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.slice_ref_
    }
}

pub struct IoSliceMut<'a, T, R>
where
    T: Clone,
    R: TrReclaimIoSliceMut<T>,
{
    slice_mut_: &'a mut [T],
    reclaimer_: Option<R>,
}

impl<'a, T, R> IoSliceMut<'a, T, R>
where
    T: Clone,
    R: TrReclaimIoSliceMut<T>,
{
    pub fn new_with_reclaimer(
        slice_mut: &'a mut [T],
        reclaimer: R,
    ) -> Self {
        IoSliceMut {
            slice_mut_: slice_mut,
            reclaimer_: Option::Some(reclaimer),
        }
    }

    pub fn new(slice: &'a mut [T]) -> Self {
        IoSliceMut {
            slice_mut_: slice,
            reclaimer_: Option::None,
        }
    }
}

impl<'a, T, R> Drop for IoSliceMut<'a, T, R>
where
    T: Clone,
    R: TrReclaimIoSliceMut<T>,
{
    fn drop(&mut self) {
        let Option::Some(r) = self.reclaimer_.take() else {
            return;
        };
        r.reclaim_mut(self)
    }
}

impl<'a, T, R> Deref for IoSliceMut<'a, T, R>
where
    T: Clone,
    R: TrReclaimIoSliceMut<T>,
{
    type Target = [T];

    fn deref(&self) -> &[T] {
        self.slice_mut_
    }
}

impl<'a, T, R> DerefMut for IoSliceMut<'a, T, R>
where
    T: Clone,
    R: TrReclaimIoSliceMut<T>,
{
    fn deref_mut(&mut self) -> &mut [T] {
        self.slice_mut_
    }
}
