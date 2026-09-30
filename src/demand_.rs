use core::{cmp, ops::{Bound, RangeBounds}};

/// 描述一次操作可接受的「数量」范围。
///
/// 取值集合一律是左闭右开区间或单边约束（见 [`DemandRange`] 的说明），`min()` / `max()`
/// 给出**含端点**的约束边界：
///
/// * [`Demand::exactly`]：恰好 n 个；
/// * [`Demand::no_more_than`]：最多 n 个（含端点）；
/// * [`Demand::at_least`]：至少 n 个；
/// * [`Demand::between`]：a 到 b（两端都含）。
#[derive(Clone, Debug)]
pub struct Demand<T = usize>(DemandRange<T>);

impl<T> Demand<T> {
    /// 借用形式的需求视图：把内部的引用而不是值放进 `Demand<&T>`。
    ///
    /// 已知局限（见 dev-notes D11）：`min` / `max` 需要 `T: funty::Integral`，而 `&T`
    /// 不满足该约束，因此当前 `Demand<&T>` 上没有任何可用的访问器，本方法事实上不可用，
    /// 待与访问器的约束位置一并决策。
    pub const fn as_ref(&self) -> Demand<&T> {
        use DemandRange::*;

        match &self.0 {
            AtLeast(l) => Demand(AtLeast(l)),
            NoMoreThan(u) => Demand(NoMoreThan(u)),
            BaseAndLen(l, u) => Demand(BaseAndLen(l, u)),
        }
    }
}

impl<T> Demand<T>
where
    T: funty::Unsigned,
{
    /// 构造「只允许一个取值」的需求：集合 `{val}`。
    ///
    /// 内部表示为 `[val, val + 1)`（起点 + 长度 1），因此 `val` 取到 `usize::MAX`
    /// 也能精确表示——这正是用「起点 + 长度」取代「上下限」的动机。
    ///
    /// ## Example
    /// ```
    /// use abs_buff::Demand;
    ///
    /// let a = Demand::exactly(2usize);
    /// assert_eq!(a.min(), Option::Some(2));
    /// assert_eq!(a.max(), Option::Some(2));
    /// assert_eq!(a.len(), 1);
    ///
    /// // 取值上限本身同样能精确表示
    /// let m = Demand::exactly(usize::MAX);
    /// assert_eq!(m.min(), Option::Some(usize::MAX));
    /// assert_eq!(m.max(), Option::Some(usize::MAX));
    /// assert_eq!(m.len(), 1);
    /// ```
    pub const fn exactly(val: T) -> Self {
        Demand(DemandRange::BaseAndLen(val, T::ONE))
    }

    pub fn less_than(val: T) -> Self {
        if val > T::ZERO {
            Demand(DemandRange::NoMoreThan(val - T::ONE))
        } else {
            Demand(DemandRange::BaseAndLen(T::ZERO, T::ZERO))
        }
    }

    /// 允许的取值个数（取值集合的元素个数）。
    ///
    /// * `exactly(2)` → 1（集合 `{2}`）；
    /// * `no_more_than(2)` → 3（集合 `{0, 1, 2}`）；
    /// * `between(1, 100)` → 100；
    /// * `at_least(5)` 的元素个数超出 `usize` 表示范围，按 `usize::MAX` 饱和。
    ///
    /// 注意 `at_least(T::MAX)` 只含一个取值 `{T::MAX}`，因此返回 1。
    /// no_more_than(T::MAX) 的取值数量与数学直觉不同，依旧是 T::MAX
    pub fn len(&self) -> T {
        use DemandRange::*;

        match self.0 {
            // {l, ..., usize::MAX}：个数 = MAX - l + 1，l == 0 时上溢 → 饱和
            AtLeast(l) =>
                if l > T::ZERO {
                    (T::MAX - l) + T::ONE
                } else {
                    T::MAX
                },
            // {0, ..., u}：个数 = u + 1，u == MAX 时上溢 → 饱和
            NoMoreThan(u) => if u < T::MAX { u + T::ONE } else { T::MAX },
            // [b, b + l)：个数就是 l
            BaseAndLen(_, l) => l,
        }
    }

    /// 由 [`RangeBounds`] 构造需求；空区间或边界溢出时返回 `Err`（原样带回边界引用）。
    ///
    /// 区间的**开闭语义照 `RangeBounds` 原样**，映射到内部表示时上界会换算成「含端点的
    /// 最大取值」：
    ///
    /// * 无上界（`..`、`5..`）映射为 [`Demand::at_least`]，不再借用 `T::MAX` 充数，
    ///   因此也不会出现 `MAX + 1` 这样的上溢；
    /// * `5..10` 映射为 `{5, ..., 9}`，`5..=10` 映射为 `{5, ..., 10}`；
    /// * 空区间（`1..1`、`..0`）与边界溢出（起点为 `Excluded(T::MAX)`）返回 `Err`。
    ///
    /// ## Example
    /// ```
    /// use abs_buff::Demand;
    ///
    /// // 左闭右开：5..10 → {5, ..., 9}
    /// let a = Demand::try_from_usize_range(&(5usize..10)).unwrap();
    /// assert_eq!(a.min(), Option::Some(5));
    /// assert_eq!(a.max(), Option::Some(9));
    /// assert_eq!(a.len(), 5);
    ///
    /// // 无上界 → {5, ...}
    /// let b = Demand::try_from_usize_range(&(5usize..)).unwrap();
    /// assert_eq!(b.max(), Option::None);
    /// ```
    pub fn try_from_usize_range(
        range: &impl RangeBounds<T>,
    ) -> Result<Self, (Bound<&T>, Bound<&T>)> {
        use Bound::*;

        let start_bound = range.start_bound();
        let end_bound = range.end_bound();

        // 起点换算成「含端点的下界」；`Excluded(MAX)` 上溢 → `None` → Err。
        let start = match start_bound {
            Included(&v) => Option::Some(v),
            Excluded(&v) => v.checked_add(T::ONE),
            Unbounded => Option::Some(T::MIN),
        };
        // 上界换算成「含端点的最大取值」：
        // * `None`            —— 无上界（映射为 `AtLeast`）；
        // * `Some(None)`      —— 空区间（例如 `..0`，`Excluded(0)` 减不出值）；
        // * `Some(Some(u))`   —— 含端点的最大取值 u。
        let last: Option<Option<T>> = match end_bound {
            Included(&v) => Option::Some(Option::Some(v)),
            Excluded(&v) => Option::Some(v.checked_sub(T::ONE)),
            Unbounded => Option::None,
        };

        match (start, last) {
            (Option::Some(l), Option::Some(Option::Some(u))) if l <= u => {
                Ok(Demand::between(l, u))
            }
            (Option::Some(l), Option::None) => Ok(Demand::at_least(l)),
            _ => Err((start_bound, end_bound)),
        }
    }
}

impl<T> Demand<T>
where
    T: funty::Integral,
{
    /// 构造「取值落在 `[min(a, b), max(a, b)]`」的需求：**两端都含**，参数顺序无关。
    ///
    /// `a == b` 时退化为只含一个取值的需求（等价于 [`Demand::exactly`]）。
    ///
    /// ## Example
    /// ```
    /// use abs_buff::Demand;
    ///
    /// // 参数顺序无关
    /// let a = Demand::between(10usize, 1);
    /// assert_eq!(a.min(), Option::Some(1));
    /// assert_eq!(a.max(), Option::Some(10));
    /// assert_eq!(a.len(), 10);
    ///
    /// // 退化成一个取值
    /// let b = Demand::between(10usize, 10);
    /// assert_eq!(b.min(), Option::Some(10));
    /// assert_eq!(b.max(), Option::Some(10));
    /// assert_eq!(b.len(), 1);
    ///
    /// // 上探到取值上限：集合 {0, ..., MAX} 等价于"无上界"
    /// let c = Demand::between(0usize, usize::MAX);
    /// assert_eq!(c.min(), Option::Some(0));
    /// assert_eq!(c.max(), Option::None);
    /// ```
    pub fn between(a: T, b: T) -> Self {
        use DemandRange::*;

        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
        if hi - lo < T::MAX {
            Demand(BaseAndLen(lo, hi - lo + T::ONE))
        } else {
            Demand(AtLeast(lo))
        }
    }

    /// 构造「至少 val」的需求：集合 `{val, val + 1, ...}`，无上界。
    ///
    /// ## Example
    /// ```
    /// use abs_buff::Demand;
    ///
    /// let a = Demand::at_least(2usize);
    /// assert_eq!(a.min(), Option::Some(2));
    /// assert_eq!(a.max(), Option::None);
    /// ```
    pub const fn at_least(val: T) -> Self {
        Demand(DemandRange::AtLeast(val))
    }

    /// 构造「最多 val」的需求：集合 `{0, 1, ..., val}`；上界**含端点**，下界无约束。
    ///
    /// 搬移/读写实现可以直接把它的 `max()` 当作"最多能搬多少"来用。
    ///
    /// ## Example
    /// ```
    /// use abs_buff::Demand;
    ///
    /// let a = Demand::no_more_than(2usize);
    /// assert_eq!(a.min(), Option::None);
    /// assert_eq!(a.max(), Option::Some(2));
    /// assert_eq!(a.len(), 3);
    /// ```
    pub const fn no_more_than(val: T) -> Self {
        Demand(DemandRange::NoMoreThan(val))
    }

    /// 取值集合的**含端点**最小值；`None` 表示"无下界"（隐含从 0 起算）。
    ///
    /// 例如 `exactly(2)`、`between(1, 5)`、`at_least(3)` 返回各自的 `Some(下界)`，
    /// 而 `no_more_than(4)` 返回 `None`。
    pub const fn min(&self) -> Option<T> {
        use DemandRange::*;

        match self.0 {
            AtLeast(l) => Option::Some(l),
            BaseAndLen(l, _) => Option::Some(l),
            _ => Option::None,
        }
    }

    /// 取值集合的**含端点**最大值；`None` 表示"无上界"。
    ///
    /// 注意这是含端点的最大取值，而不是开区间上界：`exactly(2)` 与 `no_more_than(2)`
    /// 都返回 `Some(2)`。
    pub fn max(&self) -> Option<T>
    where
        T: funty::Integral,
    {
        use DemandRange::*;

        match self.0 {
            NoMoreThan(u) => Option::Some(u),
            // `[b, b + l)` 的最大取值是 `b + l - 1`。先算 `l - 1`（构造保证 `l >= 1`）
            // 再加 `b`，可避免 `b + l` 上溢——`exactly(usize::MAX)` 正是这一情形。
            BaseAndLen(b, l) =>
                if l > T::ZERO && b < T::MAX {
                    Option::Some(b + (l - T::ONE))
                } else if b == T::MAX && l == T::ONE {
                    Option::Some(b)
                } else {
                    Option::None
                },
            _ => Option::None,
        }
    }

    pub fn is_empty(&self) -> bool {
        use DemandRange::*;
        match self.0 {
            // 当 T::ZERO == T::MIN 时是 unsigned, 所以 no_more_than(0) 是空集
            NoMoreThan(u) => u == T::ZERO && T::ZERO == T::MIN,
            // 长度为 0 是空集
            BaseAndLen(_, l) => l == T::ZERO,
            _ => false,
        }
    }

    /// 求两个需求的交集：同时被两者允许的取值集合；无交集返回 `None`。
    ///
    /// 两集合**恰好相接**时（例如 `{5}` 与 `{0, ..., 4}`）交集为空，返回 `None`，
    /// 不会 panic。
    ///
    /// ## Example
    /// ```
    /// use abs_buff::Demand;
    ///
    /// // {3, ..., 8} ∩ {5, ..., 10} = {5, ..., 8}
    /// let a = Demand::between(3usize, 8);
    /// let b = Demand::between(5usize, 10);
    /// let c = a.compromise(&b).unwrap();
    /// assert_eq!(c.min(), Option::Some(5));
    /// assert_eq!(c.max(), Option::Some(8));
    /// assert_eq!(c.len(), 4);
    ///
    /// // {5} ∩ {0, ..., 4} = ∅：恰好相接
    /// let e = Demand::exactly(5usize);
    /// let u = Demand::no_more_than(4usize);
    /// assert!(e.compromise(&u).is_none());
    /// ```
    pub fn compromise(&self, other: &Self) -> Option<Self>
    where
        T: Clone,
    {
        use DemandRange::*;

        let lhs = self.0.clone();
        let rhs = other.0.clone();

        match (lhs, rhs) {
            // 无上界 + 无上界 → 取较大的下界
            (AtLeast(a), AtLeast(b)) => {
                Some(Demand::at_least(cmp::max(a, b)))
            }

            // 无下界 + 无下界 → 取较小的上界
            (NoMoreThan(a), NoMoreThan(b)) => {
                Some(Demand::no_more_than(cmp::min(a, b)))
            }

            // `{a, ...} ∩ {0, ..., b} = {a, ..., b}`（`a <= b` 才非空）。两种参数顺序
            // 绑定相同，合并为一个分支。长度 `b - a + 1` 只有 `a == 0 && b == MAX` 会
            // 溢出，而那个集合 `{0, ..., MAX}` 与"无上界"是同一个集合。
            (AtLeast(a), NoMoreThan(b)) | (NoMoreThan(b), AtLeast(a)) if a <= b => {
                if a == T::MIN && b == T::MAX {
                    Some(Demand(AtLeast(T::MIN)))
                } else {
                    Some(Demand(BaseAndLen(a, b - a + T::ONE)))
                }
            }

            // `{a, ...} ∩ [c, c + d)`：同样把两种参数顺序合成一个分支。
            // case 分析集中在一处，且只在 `a > c` 时才做 `a - c` 的减法，
            // 因此不会像 `c - a` 那样在无符号类型上下溢。
            (AtLeast(a), BaseAndLen(c, d)) | (BaseAndLen(c, d), AtLeast(a)) => {
                if a <= c {
                    // 右侧 `[c, c + d)` 整体落在 `{a, ...}` 内。
                    Some(Demand(BaseAndLen(c, d)))
                } else if a - c < d {
                    // 交集是 `[a, c + d)`，长度 = `d - (a - c)`。
                    Some(Demand(BaseAndLen(a, d - (a - c))))
                } else {
                    // `a >= c + d`：无交集。
                    None
                }
            }

            // `{0, ..., b} ∩ [c, c + d) = [c, min(c + d, b + 1))`（`c <= b` 才非空）。
            // 长度用饱和加法：`b == T::MAX && c == 0` 时真实长度 `MAX + 1` 无法表示，
            // 但随后与 `d` 取小，结果仍然正确。
            (NoMoreThan(b), BaseAndLen(c, d)) if c <= b => {
                Some(Demand(BaseAndLen(
                    c,
                    cmp::min(d, (b - c).saturating_add(T::ONE)),
                )))
            }
            (BaseAndLen(c, d), NoMoreThan(b)) if c <= b => {
                Some(Demand(BaseAndLen(
                    c,
                    cmp::min(d, (b - c).saturating_add(T::ONE)),
                )))
            }

            // 半开区间 + 半开区间
            (BaseAndLen(a, b), BaseAndLen(c, d)) => {
                // lhs: [a, a + b)
                // rhs: [c, c + d)
                // 不计算 a + b / c + d，避免上溢。
                let (base, len) = if a >= c {
                    // 交集起点是 a。
                    // 需要 a 落在 rhs 内：a < c + d
                    // 等价于 a - c < d，且 a >= c 保证 a - c 不会下溢。
                    if a - c >= d {
                        return None;
                    }

                    // rhs 从 a 开始还剩 d - (a - c) 个元素。
                    (a, cmp::min(b, d - (a - c)))
                } else {
                    // 交集起点是 c。
                    // 需要 c 落在 lhs 内：c < a + b
                    // 等价于 c - a < b，且 c > a 保证 c - a 不会下溢。
                    if c - a >= b {
                        return None;
                    }
                    // lhs 从 c 开始还剩 b - (c - a) 个元素。
                    (c, cmp::min(d, b - (c - a)))
                };

                if len > T::ZERO {
                    Some(Demand(BaseAndLen(base, len)))
                } else {
                    None
                }
            }

            // 无下界与无下界、无上界与无上界等已在上面覆盖，其余情况无交集
            _ => None,
        }
    }
}

impl<T> RangeBounds<T> for Demand<T>
where
    T: Eq + Ord,
{
    fn start_bound(&self) -> Bound<&T> {
        match &self.0 {
            DemandRange::AtLeast(x) => Bound::Included(x),
            DemandRange::NoMoreThan(_) => Bound::Unbounded,
            DemandRange::BaseAndLen(x, _) => Bound::Included(x),
        }
    }

    fn end_bound(&self) -> Bound<&T> {
        match &self.0 {
            DemandRange::AtLeast(_) => Bound::Unbounded,
            // `{0, ..., x}` 的含端点上界就是 `x`，可直接借用。
            DemandRange::NoMoreThan(x) => Bound::Included(x),
            // 已知缺陷（需设计决策，见 dev-notes）：`[b, b + len)` 的开区间上界是
            // `b + len`，它既没有存成字段（`Bound<&T>` 要求返回引用），也可能是
            // `T::MAX + 1` 而无法用 `T` 表示。因此这里返回的 `Excluded(len)` **不是**
            // 正确的区间端点，`Demand` 作为 `RangeBounds` 使用时应避免依赖该分支。
            DemandRange::BaseAndLen(_, x) => Bound::Excluded(x),
        }
    }
}

/// 取值区间：表示一组被允许的「数量」，一律是左闭右开区间语义下的集合。
///
/// 区间不直接存上下限，而是「起点 + 长度」或单边约束。原因：`exactly(usize::MAX)`
/// 这类集合的开区间上界是 `usize::MAX + 1`，用 `usize` 存不下；改成「起点 + 长度」后
/// `{usize::MAX}` 就是 `(usize::MAX, 1)`，可以精确表示。
#[derive(Clone, Debug)]
pub(crate) enum DemandRange<T> {
    /// `{l, l + 1, ..., T::MAX}`：只有下界，无上界
    AtLeast(T),

    /// `{0, 1, ..., u}`：只有上界（含端点），下界隐含为 0
    NoMoreThan(T),

    /// `[base, base + len)`：起点 + 长度；`len >= 1`，空集一律用 `None`/`Err` 表达，
    /// 不构造长度为 0 的区间
    BaseAndLen(T, T),
}

#[cfg(test)]
mod try_from_usize_range_tests_ {
    use core::ops::{Bound, RangeBounds};

    use crate::Demand;

    /// 校验 `RangeBounds` → `Demand` 的映射：左闭右开照原样，上界换算成含端点最大值。
    /// - 测试目标：各类 Rust 区间都能映射成正确的取值集合；无上界区间映射为"无上界"，
    ///   而不是拿 `usize::MAX` 充数（后者会让 `{0, ...}` 在上界加一时溢出）。
    /// - 测试手段：对每个区间构造需求，读出 `(min, max, len)`，与手算结果比对；其中
    ///   `max` 是**含端点**最大取值，`None` 表示该侧无约束。
    /// - 判定标准：三元组逐一相等；无上界、上探 `usize::MAX` 的行分别给出文档化的结果。
    #[test]
    fn ranges_map_to_expected_sets() {
        let max = usize::MAX;

        /// 断言区间的映射结果 `(下界, 上界, 长度)`。
        fn check_<TyRange: RangeBounds<usize>>(
            range: TyRange,
            expect: (Option<usize>, Option<usize>, usize),
            name: &str,
        ) {
            let d = Demand::try_from_usize_range(&range)
                .unwrap_or_else(|_| panic!("{name}：应当能构造出需求"));
            assert_eq!(
                (d.min(), d.max(), d.len()),
                expect,
                "{name}: (min, max, len) 与预期不符"
            );
        }

        check_(.., (Option::Some(0), Option::None, max), "..");
        check_(5usize.., (Option::Some(5), Option::None, max - 4), "5..");
        check_(..10usize, (Option::Some(0), Option::Some(9), 10), "..10");
        check_(..=10usize, (Option::Some(0), Option::Some(10), 11), "..=10");
        check_(5usize..10, (Option::Some(5), Option::Some(9), 5), "5..10");
        check_(5usize..=10, (Option::Some(5), Option::Some(10), 6), "5..=10");
        check_(5usize..=5, (Option::Some(5), Option::Some(5), 1), "5..=5");
        check_(0usize..1, (Option::Some(0), Option::Some(0), 1), "0..1");
        check_(0usize..=0, (Option::Some(0), Option::Some(0), 1), "0..=0");
        check_(max..=max, (Option::Some(max), Option::Some(max), 1), "MAX..=MAX");
        check_(max.., (Option::Some(max), Option::None, 1), "MAX..");

        // `0..=MAX` 是唯一无法用"起点 + 长度"表示的集合（长度 MAX + 1），
        // 它与"无上界"等价，因此 max 为 None 而长度饱和到 MAX。
        let full = Demand::try_from_usize_range(&(0usize..=max)).unwrap();
        assert_eq!((full.min(), full.max()), (Option::Some(0), Option::None));
        assert_eq!(full.len(), max);
    }

    /// 空区间、非法边界与边界溢出必须返回 `Err`，并原样带回两端的引用。
    /// - 测试目标：`try_from_usize_range` 对空集合与溢出边界报错，而不是 panic 或静默构造
    ///   出空需求。
    /// - 测试手段：`1..1`、`5..5`、`..0`、`1..0`，以及自定义的
    ///   `(Excluded(10), Included(5))` 与起点 `Excluded(usize::MAX)`（加一溢出）。
    /// - 判定标准：全部返回 `Err`；`Err` 中带回的边界与输入一致。
    #[test]
    fn empty_or_overflowing_ranges_return_err() {
        assert!(Demand::try_from_usize_range(&(1usize..1)).is_err());
        assert!(Demand::try_from_usize_range(&(5usize..5)).is_err());
        assert!(Demand::try_from_usize_range(&(..0usize)).is_err());
        // 起点大于终点：故意用变量写，避免 `clippy::reversed_empty_ranges` 对字面量的误报。
        let (from, to) = (1usize, 0usize);
        assert!(Demand::try_from_usize_range(&(from..to)).is_err());

        struct InvalidRange;
        impl RangeBounds<usize> for InvalidRange {
            fn start_bound(&self) -> Bound<&usize> {
                Bound::Included(&10)
            }
            fn end_bound(&self) -> Bound<&usize> {
                Bound::Excluded(&5)
            }
        }
        let err = Demand::try_from_usize_range(&InvalidRange).unwrap_err();
        assert!(matches!(err.0, Bound::Included(&10)));
        assert!(matches!(err.1, Bound::Excluded(&5)));

        // 起点为 `Excluded(usize::MAX)` → 加一溢出 → Err
        struct OverflowingStart;
        impl RangeBounds<usize> for OverflowingStart {
            fn start_bound(&self) -> Bound<&usize> {
                Bound::Excluded(&usize::MAX)
            }
            fn end_bound(&self) -> Bound<&usize> {
                Bound::Unbounded
            }
        }
        assert!(Demand::try_from_usize_range(&OverflowingStart).is_err());
    }

    /// `Demand` 作为 `RangeBounds` 的视图：无约束的一侧必须是 `Unbounded`。
    /// - 测试目标：`at_least` / `no_more_than` 的 `RangeBounds` 视图要准确（单边约束可以
    ///   精确表达；`BaseAndLen` 的开区间上界没有存成字段，属已知缺陷，见 dev-notes）。
    /// - 测试手段：对 `at_least(5)` 与 `no_more_than(7)` 取 `start_bound` / `end_bound`。
    /// - 判定标准：`at_least(5)` → `[Included(5), ...)` 且上界 `Unbounded`；
    ///   `no_more_than(7)` → 下界 `Unbounded`、上界 `Included(7)`（含端点 7）。
    #[test]
    fn range_bounds_view_is_exact_for_single_sided_demands() {
        let a = Demand::at_least(5usize);
        assert!(matches!(a.start_bound(), Bound::Included(&5)));
        assert!(matches!(a.end_bound(), Bound::Unbounded));

        let n = Demand::no_more_than(7usize);
        assert!(matches!(n.start_bound(), Bound::Unbounded));
        assert!(matches!(n.end_bound(), Bound::Included(&7)));
    }

    /// `a..b` / `a..=b` 与 `between` 的等价关系必须成立。
    /// - 测试目标：两条构造路径（区间映射与直接构造）给出同一集合，特别防止把"开区间上界"
    ///   直接当成"含端点上界"塞给 `between`（那会多算一个取值）。
    /// - 测试手段：对若干 `(a, b)` 比较 `try_from_usize_range(&(a..=b))` 与
    ///   `between(a, b)`，以及 `try_from_usize_range(&(a..b))` 与 `between(a, b - 1)`。
    /// - 判定标准：`(min, max, len)` 三元组完全一致。
    #[test]
    fn ranges_agree_with_between() {
        for &(a, b) in &[
            (0usize, 0usize),
            (0, 1),
            (1, 100),
            (7, 7),
            (5, 10),
            (0, 99),
        ] {
            let from_inclusive = Demand::try_from_usize_range(&(a..=b)).unwrap();
            let direct = Demand::between(a, b);
            assert_eq!(
                (from_inclusive.min(), from_inclusive.max(), from_inclusive.len()),
                (direct.min(), direct.max(), direct.len()),
                "{a}..={b} 应当等价于 between({a}, {b})"
            );

            if a < b {
                let from_exclusive = Demand::try_from_usize_range(&(a..b)).unwrap();
                let direct_prev = Demand::between(a, b - 1);
                assert_eq!(
                    (
                        from_exclusive.min(),
                        from_exclusive.max(),
                        from_exclusive.len(),
                    ),
                    (direct_prev.min(), direct_prev.max(), direct_prev.len()),
                    "{a}..{b} 应当等价于 between({a}, {})",
                    b - 1
                );
            }
        }
    }
}

#[cfg(test)]
mod accessor_tests_ {
    use crate::Demand;

    /// 校验 `min` / `max` / `len` 与各构造器给出的取值集合一致。
    /// - 测试目标：三种表示（单边约束、起点 + 长度、单点）的访问器都要正确；单边约束的
    ///   `len` 尤其容易少算一个（`no_more_than(u)` 含 `u + 1` 个取值）。
    /// - 测试手段：对四种构造器逐一读出 `(min, max, len)` 三元组。
    /// - 判定标准：与手算的集合完全一致，包括取值上限处的极端点（不 panic、不饱和错位）。
    #[test]
    fn accessors_match_the_value_sets() {
        let max = usize::MAX;

        let e = Demand::exactly(3usize);
        assert_eq!((e.min(), e.max(), e.len()), (Option::Some(3), Option::Some(3), 1));

        let b = Demand::between(4usize, 9usize);
        assert_eq!((b.min(), b.max(), b.len()), (Option::Some(4), Option::Some(9), 6));

        // `{0, 1, 2}`：下界"无约束"故 min 为 None；个数是 u + 1 而不是 u。
        let n = Demand::no_more_than(2usize);
        assert_eq!((n.min(), n.max(), n.len()), (Option::None, Option::Some(2), 3));
        assert_eq!(Demand::no_more_than(0usize).len(), 1, "只含 0 这一个取值");

        // `{5, ...}`：个数是 MAX - 5 + 1。
        let a = Demand::at_least(5);
        assert_eq!(
            (a.min(), a.max(), a.len()),
            (Option::Some(5), Option::None, max - 4)
        );
    }

    /// 校验取值上限处的极端点：既不 panic，也不把单点集合算成空集或"无上界"。
    /// - 测试目标：`exactly(usize::MAX)`、`at_least(usize::MAX)`、`no_more_than(usize::MAX)`
    ///   与 `between(0, usize::MAX)` 的行为；旧实现会在这里上溢或算出 `len == 0`。
    /// - 测试手段：逐一构造并读出 `(min, max, len)`。
    /// - 判定标准：`exactly(MAX)` → `(MAX, MAX, 1)`；`at_least(MAX)` → `(MAX, None, 1)`；
    ///   `no_more_than(MAX)` → `(None, MAX, MAX)`（`MAX + 1` 个取值饱和为 `MAX`）；
    ///   `between(0, MAX)` 与"无上界"同集合 → `(0, None, MAX)`。
    #[test]
    fn extremes_do_not_overflow() {
        let max = usize::MAX;

        let e = Demand::exactly(max);
        assert_eq!(
            (e.min(), e.max(), e.len()),
            (Option::Some(max), Option::Some(max), 1)
        );

        let a = Demand::at_least(max);
        assert_eq!((a.min(), a.max(), a.len()), (Option::Some(max), Option::None, 1));

        let n = Demand::no_more_than(max);
        assert_eq!((n.min(), n.max(), n.len()), (Option::None, Option::Some(max), max));

        let f = Demand::between(0, max);
        assert_eq!((f.min(), f.max(), f.len()), (Option::Some(0), Option::None, max));

        // 起点与终点都取上限：必须保留有界表示，`max()` 不能变成"无上界"
        let t = Demand::between(max, max);
        assert_eq!((t.min(), t.max(), t.len()), (Option::Some(max), Option::Some(max), 1));
    }
}

#[cfg(test)]
mod compromise_tests_ {
    use std::{
        panic::{AssertUnwindSafe, catch_unwind},
        vec,
        vec::Vec,
    };

    use crate::Demand;

    /// 把需求表示成 `(含端点下界, 含端点上界)`；`None` 表示该侧无约束。
    fn bounds_(d: &Demand<usize>) -> (Option<usize>, Option<usize>) {
        (d.min(), d.max())
    }

    /// 表驱动校验 `compromise` 的交集语义（含对称性与"恰好相接即空"）。
    /// - 测试目标：任意两个需求求交都给出精确交集；交集为空——尤其两集合恰好相接——时返回
    ///   `None` 而不 panic；全程不因 `usize` 加减而上下溢。
    /// - 测试手段：逐行给出 `(说明, a, b, 期望的 (下界, 上界))`，正反两个方向各求一次交，
    ///   并用 `catch_unwind` 把 panic 变成可读的失败。
    /// - 判定标准：期望 `None` 时必须得到 `None`；期望区间时必须拿到一致的 `(下界, 上界)`；
    ///   任一方向 panic 即失败。
    #[test]
    fn compromise_intersects_requirements() {
        use Demand as D;

        type Expect = Option<(Option<usize>, Option<usize>)>;
        let no: Expect = Option::None;
        /// 期望值的简写：`Option::Some` 作为值绑定会被单一实例化，故用泛型函数。
        fn some_<T>(v: T) -> Option<T> {
            Option::Some(v)
        }
        let max = usize::MAX;

        let cases: Vec<(&str, Demand<usize>, Demand<usize>, Expect)> = vec![
            // -- 单边约束之间 --
            ("{5, ...} ∩ {10, ...}", D::at_least(5), D::at_least(10), some_((some_(10), Option::None))),
            ("{0..=10} ∩ {0..=5}", D::no_more_than(10), D::no_more_than(5), some_((Option::None, some_(5)))),
            ("{5, ...} ∩ {0..=10}", D::at_least(5), D::no_more_than(10), some_((some_(5), some_(10)))),
            ("{5, ...} ∩ {0..=4}：恰好相接", D::at_least(5), D::no_more_than(4), no),
            ("{0, ...} ∩ {0..=MAX}：上探取值上限", D::at_least(0), D::no_more_than(max), some_((some_(0), Option::None))),
            // -- 单边约束与「起点 + 长度」 --
            ("{5, ...} ∩ {0..=8}", D::at_least(5), D::between(0, 8), some_((some_(5), some_(8)))),
            ("{3, ...} ∩ {5..=8}：右侧被包含", D::at_least(3), D::between(5, 8), some_((some_(5), some_(8)))),
            ("{7, ...} ∩ {5..=8}：左侧截断右侧", D::at_least(7), D::between(5, 8), some_((some_(7), some_(8)))),
            ("{6, ...} ∩ {1..=5}：恰好相接", D::at_least(6), D::between(1, 5), no),
            ("{9, ...} ∩ {1..=5}：完全在右侧之上", D::at_least(9), D::between(1, 5), no),
            ("{0..=10} ∩ {5..=15}", D::no_more_than(10), D::between(5, 15), some_((some_(5), some_(10)))),
            ("{0..=5} ∩ {5..=15}：只重合一个取值", D::no_more_than(5), D::between(5, 15), some_((some_(5), some_(5)))),
            ("{0..=4} ∩ {5..=15}", D::no_more_than(4), D::between(5, 15), no),
            ("{0..=MAX} ∩ {1..=MAX-1}", D::no_more_than(max), D::between(1, max - 1), some_((some_(1), some_(max - 1)))),
            // 下面两行专门盯住"用起点 + 长度做减法"的写法：`[c, c + d)` 的 `c + d`
            // 在 `c == 1 && d == MAX` 时会溢出，所以只能走 `a - c < d` + `d - (a - c)`。
            ("{2, ...} ∩ {1..=MAX}", D::at_least(2), D::between(1, max), some_((some_(2), some_(max)))),
            ("{MAX, ...} ∩ {1..=MAX}", D::at_least(max), D::between(1, max), some_((some_(max), some_(max)))),
            // -- 「起点 + 长度」之间 --
            ("{1..=10} ∩ {5..=15}", D::between(1, 10), D::between(5, 15), some_((some_(5), some_(10)))),
            ("{1..=5} ∩ {5..=10}", D::between(1, 5), D::between(5, 10), some_((some_(5), some_(5)))),
            ("{1..=4} ∩ {6..=10}", D::between(1, 4), D::between(6, 10), no),
            ("{5..=10} ∩ {1..=5}", D::between(5, 10), D::between(1, 5), some_((some_(5), some_(5)))),
            ("{5..=10} ∩ {1..=4}", D::between(5, 10), D::between(1, 4), no),
            ("{1..=10} ∩ {3..=4}：右侧被包含", D::between(1, 10), D::between(3, 4), some_((some_(3), some_(4)))),
            // -- exactly（单点集合）--
            ("{5} ∩ {3..=8}", D::exactly(5), D::between(3, 8), some_((some_(5), some_(5)))),
            ("{5} ∩ {5}", D::exactly(5), D::exactly(5), some_((some_(5), some_(5)))),
            ("{5} ∩ {6}", D::exactly(5), D::exactly(6), no),
            ("{5} ∩ {0..=10}", D::exactly(5), D::no_more_than(10), some_((some_(5), some_(5)))),
            ("{5} ∩ {0..=4}：恰好相接", D::exactly(5), D::no_more_than(4), no),
            ("{5} ∩ {5, ...}", D::exactly(5), D::at_least(5), some_((some_(5), some_(5)))),
            ("{5} ∩ {6, ...}", D::exactly(5), D::at_least(6), no),
            // -- 取值上限处的极端点 --
            ("{MAX} ∩ {MAX, ...}", D::exactly(max), D::at_least(max), some_((some_(max), some_(max)))),
            ("{MAX} ∩ {0..=MAX}", D::exactly(max), D::no_more_than(max), some_((some_(max), some_(max)))),
            ("{MAX-1..=MAX} ∩ {MAX, ...}", D::between(max - 1, max), D::at_least(max), some_((some_(max), some_(max)))),
        ];

        for (name, a, b, expect) in cases {
            let forward = catch_unwind(AssertUnwindSafe(|| a.compromise(&b)))
                .unwrap_or_else(|_| panic!("{name}：空集必须返回 None，且不应上下溢"));
            assert_eq!(forward.as_ref().map(bounds_), expect, "{name}（正向）");

            let backward = catch_unwind(AssertUnwindSafe(|| b.compromise(&a)))
                .unwrap_or_else(|_| panic!("{name}（反向）：不应 panic"));
            assert_eq!(
                backward.as_ref().map(bounds_),
                expect,
                "{name}（反向）：求交应对称"
            );
        }
    }

    /// 校验 `(min, max, len)` 三者自洽：两侧都有约束时 `len == max - min + 1`。
    /// - 测试目标：用一条不变量覆盖所有构造路径（构造器 + `compromise` + 区间映射），把
    ///   "多算/少算一个取值"这类 off-by-one 一次兜住。
    /// - 测试手段：枚举一批需求两两求交，凡 `min` 与 `max` 都为 `Some` 就比对三元组。
    /// - 判定标准：`len` 恒等于 `max - min + 1`；交集不应是长度 0 的空区间。
    #[test]
    fn bounds_and_len_are_consistent() {
        let demands = std::vec![
            Demand::exactly(1usize),
            Demand::exactly(7usize),
            Demand::between(0usize, 1),
            Demand::between(3usize, 9),
            Demand::between(5usize, 5),
            Demand::at_least(4usize),
            Demand::no_more_than(6usize),
            Demand::no_more_than(1usize),
        ];

        for a in &demands {
            for b in &demands {
                let Option::Some(c) = a.compromise(b) else {
                    continue;
                };
                assert_ne!(c.len(), 0, "交集不应是长度 0 的空区间");
                if let (Option::Some(lo), Option::Some(hi)) = (c.min(), c.max()) {
                    assert_eq!(
                        c.len(),
                        hi - lo + 1,
                        "len 与 (min, max) 不自洽：{lo}..={hi}"
                    );
                }
            }
        }
    }
}
