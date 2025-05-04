/// The range description that may 
#[derive(Clone, Debug)]
pub struct Demand<T>(Bound<T>)
where
    T: Eq + Ord;

impl<T> Demand<T>
where
    T: Eq + Ord,
{
    pub const fn exactly(val: T) -> Self {
        Demand(Bound::Exact(val))
    }

    pub fn between(a: T, b: T) -> Self {
        Demand(if a < b {
            Bound::Range(a, b)
        } else if a == b {
            Bound::Exact(a)
        } else {
            Bound::Range(b, a)
        })
    }

    pub const fn at_least(val: T) -> Self {
        Demand(Bound::Lower(val))
    }

    pub const fn at_most(val: T) -> Self {
        Demand(Bound::Upper(val))
    }

    pub const fn least(&self) -> Option<&T> {
        match &self.0 {
            Bound::Lower(l) => Option::Some(l),
            Bound::Range(l, _)  => Option::Some(l),
            _ => Option::None,
        }
    }

    pub fn least_with(&self, x: T) -> Option<T>
    where
        T: Clone,
    {
        if let Option::Some(l) = self.least() {
            let l = l.clone();
            return if l <= x {
                Option::Some(l)
            } else {
                Option::None
            }
        }
        if let Option::Some(m) = self.most() {
            let m = m.clone();
            return if m <= x {
                Option::Some(m)
            } else {
                Option::Some(x)
            }
        }
        unreachable!()
    }

    pub const fn most(&self) -> Option<&T> {
        match &self.0 {
            Bound::Upper(u) => Option::Some(u),
            Bound::Range(_, u ) => Option::Some(u),
            _ => Option::None,
        }
    }

    pub fn most_with(&self, x: T) -> Option<T>
    where
        T: Clone,
    {
        if let Option::Some(m) = self.most() {
            let m = m.clone();
            return if m <= x {
                Option::Some(m)
            } else {
                Option::Some(x)
            }
        }
        if let Option::Some(l) = self.least() {
            let l = l.clone();
            return if l <= x {
                Option::Some(x)
            } else {
                Option::None
            }
        }
        unreachable!()
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