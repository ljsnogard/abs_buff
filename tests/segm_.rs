//! 段类型（`SegmRef` / `SegmMut`）的集成测试。
//!
//! 这里放需要 `abs_buff-testkit` 的用例：泛型断言函数
//! （`test_move_items_*`）与共享测试设备（[`TestInput`] / [`TestOutput`]）。
//! 之所以是集成测试而不是 `src/` 下的单元测试：`cargo test` 会为单元测试
//! **另外**编译一份 crate，它与 testkit 链接的那份不是同一个 crate 实例，
//! 跨实例的 trait 实现无法匹配。

use core::{mem::MaybeUninit, pin::Pin};
use std::vec::Vec;

use abs_buff::{
    Demand,
    buffer::{SegmMut, SegmReclaim, SegmRef},
};
use abs_buff_testkit::{TestInput, TestOutput};

/// 把 `dst`（`MaybeUninit` 数组）里已初始化的内容读出来。
fn read_init<T: Copy>(dst: &[MaybeUninit<T>]) -> Vec<T> {
    dst.iter()
        .map(|m| unsafe { m.assume_init_read() })
        .collect()
}

//-- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ----
// 泛型段测试：move_items_* 的 trait 默认实现（SegmRef / SegmMut）
//-- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ----
//
// 测试意图：abs_buff 把 `move_items_*` 提升为 `TrBuffSegmRef` /
// `TrBuffSegmMut` 的 trait 默认方法，任何实现者都必须满足这些默认实现的
// 语义。这里用 `buffer::segm_tests` 的泛型函数验证本 crate 自己的
// `SegmRef` / `SegmMut`：数据按序搬移、消费量正确推进、无重复无丢失。
//
// 内部执行设计：每个方向用一个"源 Vec + 目标数组"的简单存储，段持有
// `SegmReclaim` 计数器；泛型函数在段层面断言搬移数量与消费量，测试主体
// 在底层存储上断言内容按序到达、回收计数器精确提交。

/// 通过 trait 默认实现把 `SegmRef` 的全部元素搬进 `SegmMut`
/// （`move_items_to_segm` 与镜像的 `move_items_from_segm` 都验证）。
#[test]
fn segm_move_items_trait_defaults() {
    use abs_buff_testkit as t;

    // —— move_items_to_segm（源段一侧发起）——
    {
        let mut src_data: Vec<u64> = (0..16).collect();
        let mut dst_data = [MaybeUninit::<u64>::uninit(); 16];
        let mut src_consumed = 0usize;
        let mut dst_consumed = 0usize;
        let expect: Vec<u64> = (0..16).collect();
        let mut src = SegmRef::new(
            src_data.as_mut_slice(),
            SegmReclaim::new(Pin::new(&mut src_consumed)),
        );
        let mut dst = SegmMut::new(
            &mut dst_data[..],
            SegmReclaim::new(Pin::new(&mut dst_consumed)),
        );
        let moved = t::test_move_items_to_segm(&mut src, &mut dst, &expect);
        assert_eq!(moved, 16, "泛型函数应返回搬移数量");
        // 泛型函数已内部断言 src/dst 的 least_count == 0；这里再校验
        // 底层存储与回收计数（段 drop 时提交消费量）；
        drop(src);
        drop(dst);
        assert_eq!(read_init(&dst_data), expect, "内容必须按序搬入目标");
        assert_eq!(src_consumed, 16, "源段必须按消费量提交");
        assert_eq!(dst_consumed, 16, "目标段必须按消费量提交");
    }

    // —— move_items_from_segm（目标段一侧发起，镜像）——
    {
        let mut src_data: Vec<u32> = (10..26).collect();
        let mut dst_data = [MaybeUninit::<u32>::uninit(); 16];
        let mut src_consumed = 0usize;
        let mut dst_consumed = 0usize;
        let expect: Vec<u32> = (10..26).collect();
        let mut src = SegmRef::new(
            src_data.as_mut_slice(),
            SegmReclaim::new(Pin::new(&mut src_consumed)),
        );
        let mut dst = SegmMut::new(
            &mut dst_data[..],
            SegmReclaim::new(Pin::new(&mut dst_consumed)),
        );
        let moved =
            t::test_move_items_from_segm(&mut src, &mut dst, &expect);
        assert_eq!(moved, 16);
        drop(src);
        drop(dst);
        assert_eq!(read_init(&dst_data), expect, "内容必须按序搬入目标");
        assert_eq!(src_consumed, 16);
        assert_eq!(dst_consumed, 16);
    }

    // —— move_items_to_buff（源段 → 普通缓冲）——
    {
        let mut src_data: Vec<u8> = (0..16).collect();
        let mut dst_buf = [MaybeUninit::<u8>::uninit(); 16];
        let mut consumed = 0usize;
        let expect: Vec<u8> = (0..16).collect();
        let mut src = SegmRef::new(
            src_data.as_mut_slice(),
            SegmReclaim::new(Pin::new(&mut consumed)),
        );
        // SAFETY: u8 无 drop，位拷贝安全；
        let moved = unsafe {
            t::test_move_items_to_buff(&mut src, &mut dst_buf, &expect)
        };
        assert_eq!(moved, 16);
        drop(src);
        assert_eq!(read_init(&dst_buf), expect, "缓冲内容必须按序");
        assert_eq!(consumed, 16);
    }

    // —— move_items_from_buff（普通缓冲 → 目标段）——
    {
        let mut dst_data = [MaybeUninit::<usize>::uninit(); 16];
        let mut src_buf = [MaybeUninit::<usize>::uninit(); 16];
        let mut consumed = 0usize;
        let expect: Vec<usize> = (100..116).collect();
        let mut dst = SegmMut::new(
            &mut dst_data[..],
            SegmReclaim::new(Pin::new(&mut consumed)),
        );
        // SAFETY: usize 无 drop，位拷贝安全；
        let moved = unsafe {
            t::test_move_items_from_buff(&mut dst, &mut src_buf, &expect)
        };
        assert_eq!(moved, 16);
        drop(dst);
        assert_eq!(read_init(&dst_data), expect, "目标段内容必须按序");
        assert_eq!(consumed, 16);
    }
}

/// 测试 `SegmRef::move_items_to_output_async`：从段中把数据移动到
/// `TrOutput`，并正确推进段内部的 `offset_`。
#[compio::test]
async fn segm_ref_output_async_moves_data_and_advances_offset() {
    let mut data: Vec<u8> = (0..10).collect();
    let mut consumed = 0usize;
    let mut segm = SegmRef::new(
        data.as_mut_slice(),
        SegmReclaim::new(Pin::new(&mut consumed)),
    );
    let mut output = TestOutput::new();

    {
        let res = {
            let demand = Demand::less_than(6);
            let mut child = segm.as_segm_ref();
            child.move_items_to_output_async(&mut output, &demand)
                // .may_cancel_with(NonCancellableToken::new())
                .await
        };
        let moved = res.pick_left().expect("output_async should succeed");
        assert_eq!(moved, 6, "应按 demand 上界移动 6 个元素");
    }

    assert_eq!(segm.least_count(), 4, "父段应反映子段消费的 6 个元素");
    assert_eq!(output.snapshot(), (0..6).collect::<Vec<_>>(), "输出内容应有序");

    drop(segm);
    assert_eq!(consumed, 6, "段 drop 时应把消费量提交给 reclaimer");
}

/// 测试 `SegmMut::move_items_from_input_async`：从 `TrInput` 读取数据到段中，
/// 并正确推进段内部的 `offset_`。
#[compio::test]
async fn segm_mut_input_async_reads_data_and_advances_offset() {
    let mut storage = [MaybeUninit::<u8>::uninit(); 10];
    let mut consumed = 0usize;
    let mut segm = SegmMut::new(
        &mut storage[..],
        SegmReclaim::new(Pin::new(&mut consumed)),
    );
    let mut input = TestInput::new((0..10).collect());

    {
        let res = {
            let mut child = segm.as_segm_mut();
            child.move_items_from_input_async(
                &mut input,
                &Demand::less_than(7),
            ).await
        };
        let moved = res.pick_left().expect("input_async should succeed");
        assert_eq!(moved, 7, "应按 demand 上界读入 7 个元素");
    }

    assert_eq!(segm.least_count(), 3, "父段应反映子段读入的 7 个元素");

    drop(segm);
    assert_eq!(consumed, 7, "段 drop 时应把消费量提交给 reclaimer");

    let got: Vec<u8> = storage[..7]
        .iter()
        .map(|m| unsafe { m.assume_init_read() })
        .collect();
    assert_eq!(got, (0..7).collect::<Vec<_>>(), "读入内容应有序");
}
