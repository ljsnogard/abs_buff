use anylr::{abs::TrAnyLeftRight, Any};

#[derive(Debug)]
pub struct Demand<T>(Any<T, T>)
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
        Demand::between(val.clone(), val)
    }

    pub fn between(least: T, most: T) -> Self {
        Demand(if least < most {
            Any::new_both(least, most)
        } else {
            Any::new_both(most, least)
        })
    }

    pub const fn at_least(val: T) -> Self {
        Demand(Any::new_left(val))
    }

    pub const fn at_most(val: T) -> Self {
        Demand(Any::new_right(val))
    }

    pub fn least(&self) -> Option<&T> {
        self.0.as_ref().pick_left()
    }

    pub fn most(&self) -> Option<&T> {
        self.0.as_ref().pick_right()
    }
}
