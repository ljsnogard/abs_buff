use core::cmp;

/// Describes the amount of items needed to operate.
#[derive(Clone, Debug)]
pub struct Demand<T>(Bound<T>)
where
    T: Eq + Ord;

impl<T> Demand<T>
where
    T: Eq + Ord,
{
    /// Create zero-sized range with value x.
    /// 
    /// ## Example
    /// ```
    /// use abs_buff::Demand;
    /// 
    /// let a = Demand::exactly(1);
    /// assert!(matches!(a.min(), Option::Some(1)));
    /// assert!(matches!(a.max(), Option::Some(1)));
    /// ```
    pub const fn exactly(val: T) -> Self {
        Demand(Bound::Exact(val))
    }

    /// Create a range between (a, b), and the product could vary according to
    /// the actual value of a, b.
    /// 
    /// ## Example
    /// ```
    /// use abs_buff::Demand;
    /// 
    /// let a = Demand::between(10, 1);
    /// assert!(matches!(a.min(), Option::Some(1)));
    /// assert!(matches!(a.max(), Option::Some(10)));
    /// let b = Demand::between(1, 10);
    /// assert!(matches!(b.min(), Option::Some(1)));
    /// assert!(matches!(b.max(), Option::Some(10)));
    /// ```
    pub fn between(a: T, b: T) -> Self {
        Demand(if a < b {
            Bound::Range(a, b)
        } else if a == b {
            Bound::Exact(a)
        } else {
            Bound::Range(b, a)
        })
    }

    /// Create a range with specified least value
    /// 
    /// ## Example
    /// ```
    /// use abs_buff::Demand;
    /// 
    /// let a = Demand::with_min(2);
    /// assert!(matches!(a.min(), Option::Some(2)));
    /// assert!(a.max().is_none());
    /// ```
    pub const fn with_min(val: T) -> Self {
        Demand(Bound::Min(val))
    }

    /// Create a range with specified max value
    /// 
    /// ## Example
    /// ```
    /// use abs_buff::Demand;
    /// 
    /// let a = Demand::with_max(2);
    /// assert!(matches!(a.max(), Option::Some(2)));
    /// assert!(a.min().is_none());
    /// ```
    pub const fn with_max(val: T) -> Self {
        Demand(Bound::Max(val))
    }

    /// Check if a low bound is included in the demand
    pub const fn min(&self) -> Option<&T> {
        match &self.0 {
            Bound::Min(l) => Option::Some(l),
            Bound::Exact(x) => Option::Some(x),
            Bound::Range(l, _)  => Option::Some(l),
            _ => Option::None,
        }
    }

    pub const fn max(&self) -> Option<&T> {
        match &self.0 {
            Bound::Max(u) => Option::Some(u),
            Bound::Exact(x) => Option::Some(x),
            Bound::Range(_, u ) => Option::Some(u),
            _ => Option::None,
        }
    }

    pub const fn as_ref(&self) -> Demand<&T> {
        match &self.0 {
            Bound::Min(l) => Demand(Bound::Min(l)),
            Bound::Max(u) => Demand(Bound::Max(u)),
            Bound::Range(l, u) => Demand(Bound::Range(l, u)),
            Bound::Exact(v) => Demand(Bound::Exact(v)),
        }
    }

    /// Create a narrowed range from the least side if x is within the range
    ///
    /// ## Example
    /// ```
    /// use abs_buff::Demand;
    ///
    /// let a = Demand::between(5, 10);
    /// assert!(a.min().is_some_and(|l| *l == 5));
    /// let narrowed = a.narrow_from_min(8).unwrap();
    /// assert!(narrowed.min().is_some_and(|l| *l == 8));
    ///
    /// let b = Demand::with_max(10);
    /// assert!(b.min().is_none());
    /// let narrowed = b.narrow_from_min(7).unwrap();
    /// assert!(narrowed.min().is_some_and(|l| *l == 7));
    /// ```
    pub fn narrow_from_min(&self, x: T) -> Option<Self>
    where
        T: Clone,
    {
        match &self.0 {
            Bound::Min(l) if l <= &x
                => Option::Some(Demand::with_min(x)),
            Bound::Max(u) if &x <= u
                => Option::Some(Demand::between(x, u.clone())),
            Bound::Range(l, u) if l <= &x && &x <= u
                => Option::Some(Demand::between(cmp::max(l.clone(), x), u.clone())),
            Bound::Exact(v) if &x == v
                => Option::Some(Demand::exactly(x)),
            _ => Option::None,
        }
    }

    /// Create a narrowed range from the max side if x is within the range
    ///
    /// ## Example
    /// ```
    /// use abs_buff::Demand;
    /// 
    /// let a = Demand::between(5, 10);
    /// assert!(a.max().is_some_and(|m| *m == 10));
    /// let narrowed = a.narrow_from_max(8).unwrap();
    /// assert!(narrowed.max().is_some_and(|m| *m == 8));
    ///
    /// let b = Demand::with_min(5);
    /// assert!(b.max().is_none());
    /// let narrowed = b.narrow_from_max(7).unwrap();
    /// assert!(narrowed.max().is_some_and(|m| *m == 7));
    /// ```
    pub fn narrow_from_max(&self, x: T) -> Option<Self>
    where
        T: Clone,
    {
        match &self.0 {
            Bound::Min(l) if l <= &x
                => Option::Some(Demand::between(l.clone(), x)),
            Bound::Max(u) if &x <= u
                => Option::Some(Demand::with_max(x)),
            Bound::Range(l, u) if l <= &x && &x <= u
                => Option::Some(Demand::between(l.clone(), cmp::min(x, u.clone()))),
            Bound::Exact(v) if &x == v
                => Option::Some(Demand::exactly(x)),
            _ => Option::None,
        }
    }
}

#[derive(Clone, Debug)]
enum Bound<T>
where
    T: Eq + Ord,
{
    Min(T),
    Max(T),
    Exact(T),
    Range(T, T),
}
