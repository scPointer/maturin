mod range_action_map;
use std::collections::btree_map::Range;

use range_action_map::*;

pub struct Seg(usize, usize);
impl Segment for Seg {
    fn remove(&mut self) {}
    fn split(&mut self, pos: CmpType) -> (Self, Self) {
        (Self(self.0, pos), Self(pos, self.1))
    }
    fn modify(&mut self, new_flag: FlagType) {}
}

pub fn test_find(ram: &mut RangeActionMap<Seg>, pos: CmpType) {
    println!("try find seg include {pos}");
    if let Some(seg) = ram.find(pos) {
        println!("find seg {} {}", seg.0, seg.1);
    } else {
        println!("seg not found");
    }
}

fn main() {
    let mut ram = RangeActionMap::<Seg>::new();
    ram.insert_raw(3, 7, Seg(3, 7));
    test_find(&mut ram, 2);
    test_find(&mut ram, 5);
    test_find(&mut ram, 7);
}
