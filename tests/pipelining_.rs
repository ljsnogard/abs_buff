//! `pipelining::join` 的集成测试。
//!
//! 这些测试依赖 `abs_buff-testkit` 提供的共享测试设备（[`TestErr`]）。之所以
//! 放在集成测试而不是 `src/` 里的单元测试：`cargo test` 会为单元测试**另外**
//! 编译一份 crate，它与 testkit 链接的那份不是同一个 crate 实例，跨实例的
//! trait 实现（如 `TrTaggedError`）无法匹配。

use abs_buff::{
    Demand, ReadySegm, TrBuffRead, TrBuffTryRead, TrBuffTryWrite, TrBuffWrite,
    buffer::{SegmMut, SegmReclaim, SegmRef, TrConsumerState, TrProducerState},
    pipelining::{PipeJoin, PipeJoinIoResult},
    x_deps::anylr::SomeOf,
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
// 测试替身：直接架在 `SegmRef` / `SegmMut` + `SegmReclaim` 上的读/写缓冲，
// 让管道端到端地走真实的段机制。
//-- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ----

/// 读端（rx）：一个存放未消费数据的 `Vec`，外加一个由借出段的
/// `SegmReclaim` 推进的消费位置。
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

impl<T> TrBuffTryRead<T> for TestRx<T> {
    type SegmRef<'f> = SegmRef<'f, T, SegmReclaim<'f>> where Self: 'f;
    type Err = TestErr;

    fn try_read<'f>(
        &'f mut self,
        demand: &'f Demand<usize>,
    ) -> SomeOf<Self::SegmRef<'f>, Self::Err> {
        let mut take = demand.max().copied().unwrap_or(usize::MAX);
        if self.chunk > 0 {
            take = core::cmp::min(take, self.chunk);
        }
        take = core::cmp::min(take, self.data.len() - self.pos);
        let buffer = &mut self.data[self.pos..self.pos + take];
        let reclaim = SegmReclaim::new(Pin::new(&mut self.pos));
        let segm = SegmRef::new(buffer, reclaim);
        SomeOf::new_left(segm)
    }
}

impl<T> TrBuffRead<T> for TestRx<T> {
    type ReadAsync<'f> = ReadySegm<SegmRef<'f, T, SegmReclaim<'f>>, TestErr>
    where
        Self: 'f;

    fn read_async<'f>(
        &'f mut self,
        demand: &'f Demand<usize>,
    ) -> Self::ReadAsync<'f> {
        // 本测试设备同步就绪：直接复用 `try_read`，避免两条路径逻辑漂移。
        ReadySegm::new(<Self as TrBuffTryRead<T>>::try_read(self, demand))
    }
}

/// 写端（tx）：一块固定容量的 `MaybeUninit` 存储；数据写入时由借出段的
/// `SegmReclaim` 推进写入位置。
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

    /// 目前已按序写入的元素。
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

impl<T> TrProducerState for TestTx<T> {
    fn producer_state(&self) -> Option<(usize, bool)> {
        let s = self.buff.len() - self.pos;
        Option::Some((s, s == 0))
    }
}

impl<T> TrBuffTryWrite<T> for TestTx<T> {
    type SegmMut<'f> = SegmMut<'f, T, SegmReclaim<'f>> where Self: 'f;

    type Err = TestErr;

    fn try_write<'f>(
        &'f mut self,
        demand: &'f Demand<usize>,
    ) -> SomeOf<Self::SegmMut<'f>, Self::Err> {
        let free = self.buff.len() - self.pos;
        if free == 0 {
            return SomeOf::new_right(TestErr::Stuffed);
        }
        let take = core::cmp::min(
            demand.max().copied().unwrap_or(usize::MAX),
            free,
        );
        let segm = SegmMut::new(
            &mut self.buff[self.pos..self.pos + take],
            SegmReclaim::new(Pin::new(&mut self.pos)),
        );
        SomeOf::new_left(segm)
    }
}

impl<T> TrBuffWrite<T> for TestTx<T> {
    type WriteAsync<'f> = ReadySegm<Self::SegmMut<'f>, TestErr> where Self: 'f;

    fn write_async<'f>(
        &'f mut self,
        demand: &'f Demand<usize>,
    ) -> Self::WriteAsync<'f> {
        // 本测试设备同步就绪：直接复用 `try_write`，避免两条路径逻辑漂移。
        ReadySegm::new(<Self as TrBuffTryWrite<T>>::try_write(self, demand))
    }
}

/// 在没有执行器的前提下把 future 轮询到完成；本文件用到的 future 都在
/// 第一次 poll 时就绪。
fn block_on<F: Future>(fut: F) -> F::Output {
    let mut fut = core::pin::pin!(fut);
    let waker = Waker::noop();
    let mut cx = Context::from_waker(waker);
    match fut.as_mut().poll(&mut cx) {
        Poll::Ready(v) => v,
        Poll::Pending => {
            panic!("管道 future 必须在第一次 poll 时就绪")
        }
    }
}

//-- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ----
// PipeJoin 行为
//-- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ----

/// 验证正常路径：读端的全部可读数据按序搬进写端，并以精确数量报告 `RxDrained`。
/// - 手段：读端放入 100 字节且标记为关闭，写端容量给足，`block_on` 一次
///   `pipe_async`。
/// - 判断：结果为 `RxDrained(100)`；读端 `pos` 推进到 100；写端收到的字节序列
///   与输入完全一致。
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
    assert_eq!(rx.pos, TOTAL, "读端必须消费全部数据");
    assert_eq!(
        tx.collected(),
        expected,
        "写端必须按序收到全部数据"
    );
}

/// 验证读端分块借出时，每次 `read_async` 交出的都是紧接上一块的下一段内容。
/// - 手段：读端放入 100 字节、块大小设为 30，写端容量给足，跑完 `pipe_async`。
/// - 判断：结果为 `RxDrained(100)`，且写端收到的序列与输入完全一致（若分块
///   错位，序列会在拼接处不匹配）。
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

/// 验证中途写端塞满时报告 `TxErr` 并准确定位断点，换一个更大的写端续传后
/// 不重不漏。
/// - 手段：读端 100 字节；先用容量 16 的写端跑一次，再用容量足够的写端对同一
///   读端续跑一次。
/// - 判断：首次为 `TxErr { count: 16, err: Stuffed }`、读端 `pos` 停在 16、
///   写端恰为前 16 字节；续跑为 `RxDrained(84)`，第二个写端恰为后 84 字节。
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
        "必须恰好只搬走一个写入块"
    );
    // 读端停在被搬走的那一块之后……
    assert_eq!(rx.pos, TX_CAP);
    // ……写端持有的也恰好是这一块。
    assert_eq!(tx.collected(), expected[..TX_CAP]);

    // 换一个容量足够的写端续传：剩余数据恰好到达一次。
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

/// 验证写端一开始就写满时报告 `TxBlocked`，且不消费读端任何数据。
/// - 手段：读端 3 字节，写端容量为 0，跑一次 `pipe_async`。
/// - 判断：结果为 `TxBlocked(0)`，读端 `pos` 仍为 0。
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
        "写端阻塞时不得消费任何数据"
    );
}

/// 验证读端一开始就已枯竭时报告 `RxDrained(0)`，且不向写端写入任何数据。
/// - 手段：空读端、容量 8 的写端，跑一次 `pipe_async`。
/// - 判断：结果为 `RxDrained(0)`，写端 `pos` 仍为 0。
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
        "读端枯竭时不得写入任何数据"
    );
}

/// 验证零尺寸元素类型会让管道短路为 `NoOps`。
/// - 手段：读写端都以 `()` 作为元素类型，跑一次 `pipe_async`。
/// - 判断：结果为 `NoOps`，读端与写端的 `pos` 都保持 0。
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
