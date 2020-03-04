use crate::Component;

pub trait Query {}

impl<'a, A> Query for &'a A where A: Component {}
