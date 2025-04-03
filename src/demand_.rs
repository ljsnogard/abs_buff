enum Amount<T> {
    AtLeast(T),
    AtMost(T),
    Between(T, T),
}

pub struct Demand<T>(Amount<T>)
where
    T: PartialOrd;

impl<T> Demand<T>
where
    T: PartialOrd,
{
    pub fn exactly(val: T) -> Self
    where
        T: Clone,
    {
        Demand(Amount::Between(val.clone(), val))
    }

    pub fn between(least: T, most: T) -> Self {
        let inner = if PartialOrd::lt(&least, &most) {
            Amount::Between(least, most)
        } else {
            Amount::Between(most, least)
        };
        Demand(inner)
    }

    pub fn at_least(val: T) -> Self {
        Demand(Amount::AtLeast(val))
    }

    pub fn at_most(val: T) -> Self {
        Demand(Amount::AtMost(val))
    }

    pub fn least(&self) -> Option<&T> {
        match &self.0 {
            Amount::AtLeast(v) => Option::Some(v),
            Amount::Between(v, _) => Option::Some(v),
            _ => Option::None,
        }
    }

    pub fn most(&self) -> Option<&T> {
        match &self.0 {
            Amount::AtMost(v) => Option::Some(v),
            Amount::Between(_, v) => Option::Some(v),
            _ => Option::None,
        }
    }
}
