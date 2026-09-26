//! `pipelining::join` 的集成测试。
//!
//! 这些测试依赖 `abs_buff-testkit` 提供的共享测试设备（[`TestErr`]）。之所以
//! 放在集成测试而不是 `src/` 里的单元测试：`cargo test` 会为单元测试**另外**
//! 编译一份 crate，它与 testkit 链接的那份不是同一个 crate 实例，跨实例的
//! trait 实现（如 `TrTaggedError`）无法匹配。

use abs_buff::{
    Demand, ReadySegm, TrBuffRead, TrBuffWrite, buffer::{SegmMut, SegmReclaim, SegmRef, TrConsumerState}, pipelining::{PipeJoin, PipeJoinIoResult}, x_deps::anylr::SomeOf,
};
use core::{
    future::Future,
    mem::MaybeUninit,
    pin::Pin,
    task::{Context, Poll, Waker},
};
use std::{vec, vec::Vec};

use abs_buff_testkit::TestErr;

//-- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ----
// Test doubles: a read buffer and a write buffer built directly on
// `SegmRef` / `SegmMut` with `SegmReclaim`, so the pipe exercises the real
// segment machinery end to end.
//-- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ----

/// The read (rx) half: a `Vec` of unconsumed data plus a consumption
/// counter advanced by the `SegmReclaim` of the borrowed segments.
struct TestRx<T> {
    data: Vec<T>,
    pos: usize,
    chunk: usize,
    closed: bool,
}

impl<T> TestRx<T> {
    fn new(data: Vec<T>, closed: bool) -> Self {
        TestRx {
            data,
            pos: 0,
            chunk: 0,
            closed,
        }
    }

    fn new_chunked(data: Vec<T>, closed: bool, chunk: usize) -> Self {
        TestRx {
            data,
            pos: 0,
            chunk,
            closed,
        }
    }
}

impl<T> TrConsumerState for TestRx<T> {
    fn consumer_state(&self) -> Option<(usize, bool)> {
        Option::Some((self.data.len() - self.pos, self.closed))
    }
}

impl<T> abs_buff::TrBuffTryRead<T> for TestRx<T> {
    type SegmRef<'f> = SegmRef<'f, T, SegmReclaim<'f>> where Self: 'f;
    type Err = TestErr;

    fn try_read<'f>(
        &'f mut self,
        demand: &'f Demand<usize>,
    ) -> SomeOf<Self::SegmRef<'f>, Self::Err> {
        todo!()
    }
}

impl<T> TrBuffRead<T> for TestRx<T> {
    type ReadAsync<'f> = ReadySegm<SegmRef<'f, T, SegmReclaim<'f>>, TestErr>
    where
        Self: 'f;

    fn read_async<'f>(
        &'f mut self,
        demand: &Demand<usize>,
    ) -> Self::ReadAsync<'f> {
        let mut take = demand.max().copied().unwrap_or(usize::MAX);
        if self.chunk > 0 {
            take = core::cmp::min(take, self.chunk);
        }
        take = core::cmp::min(take, self.data.len() - self.pos);
        let buffer = &mut self.data[self.pos..self.pos + take];
        let reclaim = SegmReclaim::new(Pin::new(&mut self.pos));
        let segm = SegmRef::new(buffer, reclaim);
        ReadySegm::new(SomeOf::new_left(segm))
    }
}

/// The write (tx) half: a fixed `MaybeUninit` storage; the borrowed
/// segments advance `pos` via `SegmReclaim` as data is written into them.
struct TestTx<T> {
    buff: Vec<MaybeUninit<T>>,
    pos: usize,
}

impl<T> TestTx<T> {
    fn with_capacity(cap: usize) -> Self {
        let mut buff = Vec::with_capacity(cap);
        buff.resize_with(cap, MaybeUninit::uninit);
        TestTx { buff, pos: 0 }
    }

    /// The items actually written so far, in order.
    fn collected(&self) -> Vec<T>
    where
        T: Copy,
    {
        self.buff[..self.pos]
            .iter()
            .map(|m| unsafe { m.assume_init_read() })
            .collect()
    }
}

impl<T> abs_buff::buffer::TrProducerState for TestTx<T> {
    fn producer_state(&self) -> Option<(usize, bool)> {
        let s = self.buff.len() - self.pos;
        Option::Some((s, s == 0))
    }
}

impl<T> abs_buff::TrBuffTryWrite<T> for TestTx<T> {
    type SegmMut<'f> = SegmMut<'f, T, SegmReclaim<'f>> where Self: 'f;

    type Err = TestErr;

    fn try_write<'f>(
        &'f mut self,
        demand: &'f Demand<usize>,
    ) -> SomeOf<Self::SegmMut<'f>, Self::Err> {
        todo!()
    }
}

impl<T> TrBuffWrite<T> for TestTx<T> {
    type WriteAsync<'f> = ReadySegm<Self::SegmMut<'f>, TestErr> where Self: 'f;

    fn write_async<'f>(
        &'f mut self,
        demand: &Demand<usize>,
    ) -> Self::WriteAsync<'f> {
        let free = self.buff.len() - self.pos;
        if free == 0 {
            return ReadySegm::new(SomeOf::new_right(TestErr::Stuffed));
        }
        let take = core::cmp::min(
            demand.max().copied().unwrap_or(usize::MAX),
            free,
        );
        let segm = SegmMut::new(
            &mut self.buff[self.pos..self.pos + take],
            SegmReclaim::new(Pin::new(&mut self.pos)),
        );
        ReadySegm::new(SomeOf::new_left(segm))
    }
}

/// Poll a future to completion without an executor; all the futures used
/// here are ready on their first poll.
fn block_on<F: Future>(fut: F) -> F::Output {
    let mut fut = core::pin::pin!(fut);
    let waker = Waker::noop();
    let mut cx = Context::from_waker(waker);
    match fut.as_mut().poll(&mut cx) {
        Poll::Ready(v) => v,
        Poll::Pending => {
            panic!("the pipe future must complete on the first poll")
        }
    }
}

//-- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ----
// PipeJoin behavior
//-- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ----

/// The happy path: everything readable is moved into the writer, in order,
/// and the pipe reports `RxDrained` with the exact transferred count.
#[test]
fn pipe_transfers_all_data_and_reports_drained() {
    const TOTAL: usize = 100;
    let expected: Vec<u8> = (0..TOTAL).map(|i| (i % 256) as u8).collect();
    let mut rx = TestRx::new(expected.clone(), true);
    let mut tx = TestTx::with_capacity(TOTAL + 32);

    let result = block_on(async {
        let mut pipe = PipeJoin::new(&mut tx, &mut rx);
        pipe.pipe_async().await
    });

    assert!(matches!(result, PipeJoinIoResult::RxDrained(c) if c == TOTAL));
    assert_eq!(rx.pos, TOTAL, "the reader must consume everything");
    assert_eq!(
        tx.collected(),
        expected,
        "the writer must receive everything in order"
    );
}

/// The reader yields its data in chunks: each `read_async` must hand out
/// exactly the next chunk, and the pipe must drain all of them.
#[test]
fn pipe_reads_in_chunks_next_chunk_is_next_content() {
    const TOTAL: usize = 100;
    const CHUNK: usize = 30;
    let expected: Vec<u8> = (0..TOTAL).map(|i| (i % 256) as u8).collect();
    let mut rx = TestRx::new_chunked(expected.clone(), true, CHUNK);
    let mut tx = TestTx::with_capacity(TOTAL + 32);

    let result = block_on(async {
        let mut pipe = PipeJoin::new(&mut tx, &mut rx);
        pipe.pipe_async().await
    });

    assert!(matches!(result, PipeJoinIoResult::RxDrained(c) if c == TOTAL));
    assert_eq!(tx.collected(), expected);
}

/// A mid-transfer blockage: the writer accepts one piece, then reports
/// `Blocked`. The pipe must report `TxErr` with exactly that piece size,
/// leave the reader right after the transferred data, and allow a retry on
/// a fresh writer to transfer the rest — no duplication, no loss.
#[test]
fn pipe_partial_transfer_then_retry_no_dup_no_loss() {
    const TOTAL: usize = 100;
    const TX_CAP: usize = 16;
    let expected: Vec<u8> = (0..TOTAL).map(|i| (i % 256) as u8).collect();

    let mut rx = TestRx::new(expected.clone(), true);
    let mut tx = TestTx::with_capacity(TX_CAP);

    let result = block_on(async {
        let mut pipe = PipeJoin::new(&mut tx, &mut rx);
        pipe.pipe_async().await
    });
    assert!(
        matches!(result, PipeJoinIoResult::TxErr { count, err: TestErr::Stuffed } if count == TX_CAP),
        "exactly one write piece must be transferred"
    );
    // The reader stopped right after the transferred piece...
    assert_eq!(rx.pos, TX_CAP);
    // ...and the writer holds exactly the first piece.
    assert_eq!(tx.collected(), expected[..TX_CAP]);

    // Retry with a fresh, big-enough writer: the rest arrives, exactly once.
    let mut tx2 = TestTx::with_capacity(TOTAL + 32);
    let result2 = block_on(async {
        let mut pipe = PipeJoin::new(&mut tx2, &mut rx);
        pipe.pipe_async().await
    });
    assert!(
        matches!(result2, PipeJoinIoResult::RxDrained(c) if c == TOTAL - TX_CAP)
    );
    assert_eq!(tx2.collected(), expected[TX_CAP..]);
}

/// The writer is already full before the pipe starts: report `TxBlocked`
/// without consuming anything.
#[test]
fn pipe_blocked_tx_reports_blocked_without_consuming() {
    let mut rx = TestRx::new(vec![1u8, 2, 3], true);
    let mut tx = TestTx::with_capacity(0);

    let result = block_on(async {
        let mut pipe = PipeJoin::new(&mut tx, &mut rx);
        pipe.pipe_async().await
    });

    assert!(matches!(result, PipeJoinIoResult::TxBlocked(0)));
    assert_eq!(
        rx.pos, 0,
        "nothing must be consumed when the writer is blocked"
    );
}

/// The reader is already drained before the pipe starts: report
/// `RxDrained(0)`.
#[test]
fn pipe_drained_rx_reports_drained() {
    let mut rx = TestRx::new(Vec::<u8>::new(), true);
    let mut tx = TestTx::with_capacity(8);

    let result = block_on(async {
        let mut pipe = PipeJoin::new(&mut tx, &mut rx);
        pipe.pipe_async().await
    });

    assert!(matches!(result, PipeJoinIoResult::RxDrained(0)));
    assert_eq!(
        tx.pos, 0,
        "nothing must be written when the reader is drained"
    );
}

/// A zero-sized item type short-circuits the pipe into `NoOps`.
#[test]
fn pipe_zst_returns_no_ops() {
    let mut rx = TestRx::new(vec![(); 4], true);
    let mut tx = TestTx::with_capacity(4);

    let result = block_on(async {
        let mut pipe = PipeJoin::new(&mut tx, &mut rx);
        pipe.pipe_async().await
    });

    assert!(matches!(result, PipeJoinIoResult::NoOps));
    assert_eq!(rx.pos, 0);
    assert_eq!(tx.pos, 0);
}
