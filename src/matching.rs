use std::marker::PhantomData;
use std::time::Duration;

pub trait Condition {}

pub trait Matcher<T> 
where
    {
    type Condition;

    fn matches(&self, c: Self::Condition) -> bool;
}

pub(crate) struct Seen {}
impl Seen {
    fn from_duration(d: Duration) -> ProcLifetime<Seen> {
        ProcLifetime {
            span: d,
            ty: PhantomData {},
        }
    }
}

pub(crate) struct NotSeen {}
impl NotSeen {
    fn from_duration(d: Duration) -> ProcLifetime<NotSeen> {
        ProcLifetime {
            span: d,
            ty: PhantomData {},
        }
    }
}

pub(crate) struct ProcLifetime<CondType> {
    pub span: Duration,
    ty: PhantomData<CondType>,
}
impl<T> Condition for ProcLifetime<T> {}
