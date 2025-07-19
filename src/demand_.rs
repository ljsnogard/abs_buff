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
    /// assert!(matches!(a.least(), Option::Some(1)));
    /// assert!(matches!(a.most(), Option::Some(1)));
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
    /// assert!(matches!(a.least(), Option::Some(1)));
    /// assert!(matches!(a.most(), Option::Some(10)));
    /// let b = Demand::between(1, 10);
    /// assert!(matches!(b.least(), Option::Some(1)));
    /// assert!(matches!(b.most(), Option::Some(10)));
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
    /// let a = Demand::at_least(2);
    /// assert!(matches!(a.least(), Option::Some(2)));
    /// assert!(a.most().is_none());
    /// ```
    pub const fn at_least(val: T) -> Self {
        Demand(Bound::Lower(val))
    }

    /// Create a range with specified max value
    /// 
    /// ## Example
    /// ```
    /// use abs_buff::Demand;
    /// 
    /// let a = Demand::at_most(2);
    /// assert!(matches!(a.most(), Option::Some(2)));
    /// assert!(a.least().is_none());
    /// ```
    pub const fn at_most(val: T) -> Self {
        Demand(Bound::Upper(val))
    }

    /// Check if a low bound is included in the demand
    pub const fn least(&self) -> Option<&T> {
        match &self.0 {
            Bound::Lower(l) => Option::Some(l),
            Bound::Exact(x) => Option::Some(x),
            Bound::Range(l, _)  => Option::Some(l),
            _ => Option::None,
        }
    }

    pub const fn most(&self) -> Option<&T> {
        match &self.0 {
            Bound::Upper(u) => Option::Some(u),
            Bound::Exact(x) => Option::Some(x),
            Bound::Range(_, u ) => Option::Some(u),
            _ => Option::None,
        }
    }

    pub const fn as_ref(&self) -> Demand<&T> {
        match &self.0 {
            Bound::Lower(l) => Demand(Bound::Lower(l)),
            Bound::Upper(u) => Demand(Bound::Upper(u)),
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
    /// assert!(a.least().is_some_and(|l| *l == 5));
    /// let narrowed = a.narrow_from_least(8).unwrap();
    /// assert!(narrowed.least().is_some_and(|l| *l == 8));
    ///
    /// let b = Demand::at_most(10);
    /// assert!(b.least().is_none());
    /// let narrowed = b.narrow_from_least(7).unwrap();
    /// assert!(narrowed.least().is_some_and(|l| *l == 7));
    /// ```
    pub fn narrow_from_least(self, x: T) -> Option<Self> {
        match self.0 {
            Bound::Lower(l) if l <= x
                => Option::Some(Demand::at_least(x)),
            Bound::Upper(u) if x <= u
                => Option::Some(Demand::between(x, u)),
            Bound::Range(l, u) if l <= x && x <= u
                => Option::Some(Demand::between(cmp::max(l, x), u)),
            Bound::Exact(v) if x == v
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
    /// assert!(a.most().is_some_and(|m| *m == 10));
    /// let narrowed = a.narrow_from_most(8).unwrap();
    /// assert!(narrowed.most().is_some_and(|m| *m == 8));
    ///
    /// let b = Demand::at_least(5);
    /// assert!(b.most().is_none());
    /// let narrowed = b.narrow_from_most(7).unwrap();
    /// assert!(narrowed.most().is_some_and(|m| *m == 7));
    /// ```
    pub fn narrow_from_most(self, x: T) -> Option<Self> {
        match self.0 {
            Bound::Lower(l) if l <= x
                => Option::Some(Demand::between(l, x)),
            Bound::Upper(u) if x <= u
                => Option::Some(Demand::at_most(x)),
            Bound::Range(l, u) if l <= x && x <= u
                => Option::Some(Demand::between(l, cmp::min(x, u))),
            Bound::Exact(v) if x == v
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
    Lower(T),
    Upper(T),
    Exact(T),
    Range(T, T),
}
