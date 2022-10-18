pub use super::types::*;

pub trait Segment: Sized {
    fn remove(&mut self);
    fn split(&mut self, pos: CmpType) -> (Self, Self);
    fn modify(&mut self, new_flag: FlagType);
}
