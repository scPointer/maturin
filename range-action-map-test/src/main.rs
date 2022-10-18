mod range_action_map;
use std::collections::btree_map::Range;
mod resource;
use resource::Frame;

use range_action_map::*;

#[derive(Debug)]
pub struct Seg {
    pub start: CmpType,
    pub end: CmpType,
    pub flags: PTEFlags,
    frames: Vec<Frame>,
}

impl Seg {
    pub fn new(start: CmpType, end: CmpType, flags: PTEFlags) -> Self {
        let mut frames: Vec<Frame> = Vec::new();
        for _ in start..end {
            frames.push(Frame::alloc());
        }
        Self {
            start,
            end,
            flags,
            frames 
        }
    }
}

impl Segment for Seg {
    fn remove(&mut self) {
        self.frames.clear();
    }
    fn split(&mut self, pos: CmpType) -> Self {
        let right_frames = self.frames.drain(pos-self.start..).collect();
        let old_end = self.end;
        self.end = pos;
        Self {
            start: pos,
            end: old_end,
            flags: self.flags,
            frames: right_frames,
        }
    }
    fn modify(&mut self, new_flag: IdentType) {
        self.flags = new_flag
    }
}

pub fn test_find(ram: &mut RangeActionMap<Seg>, pos: CmpType) {
    println!("try find seg include {pos}");
    if let Some(seg) = ram.find(pos) {
        println!("find seg {} {}", seg.start, seg.end);
    } else {
        println!("seg not found");
    }
}

fn main() {
}

#[test]
fn test_ram() {
    let mut ram = RangeActionMap::<Seg>::new();
    ram.insert_raw(3, 7, Seg::new(3, 7, PTE_RU()));
    test_find(&mut ram, 2);
    test_find(&mut ram, 5);
    test_find(&mut ram, 7);
    
}

#[test]
fn test_seg() {
    let mut seg = Seg::new(5,10, PTE_RU());
    seg.shrink_to_left(8);
    assert_eq!(seg.start, 5);
    assert_eq!(seg.end, 8);
    //println!("{:#?}", seg);
    seg.shrink_to_right(7);
    //println!("{:#?}", seg);
    assert_eq!(seg.start, 7);
    assert_eq!(seg.end, 8);
    let mut seg = Seg::new(1,100, PTE_RU());
    let mut rseg = seg.split_and_remove_middle(6, 13);
    assert_eq!(seg.start, 1);
    assert_eq!(seg.end, 6);
    assert_eq!(rseg.start, 13);
    assert_eq!(rseg.end, 100);
    rseg.modify(PTE_RWU());
    assert_eq!(rseg.start, 13);
    assert_eq!(rseg.end, 100);
    assert_eq!(rseg.flags, PTE_RWU());
    let rrseg = rseg.modify_left(77, PTE_RXU());
    assert_eq!(rseg.flags, PTE_RXU());
    assert_eq!(rseg.end, 77);
    assert_eq!(rrseg.flags, PTE_RWU());
    let rrseg = rseg.modify_right(72, PTE_U());
    assert_eq!(rseg.flags, PTE_RXU());
    assert_eq!(rseg.end, 72);
    assert_eq!(rrseg.flags, PTE_U());
    assert_eq!(rrseg.start, 72);
    assert_eq!(rrseg.end, 77);
    let (mrseg, rrseg) = rseg.modify_middle(33, 55, PTE_NORMAL());
    assert_eq!(rseg.flags, PTE_RXU());
    assert_eq!(rseg.end, 33);
    assert_eq!(mrseg.flags, PTE_NORMAL());
    assert_eq!(mrseg.start, 33);
    assert_eq!(mrseg.end, 55);
    assert_eq!(rrseg.flags, PTE_RXU());
    assert_eq!(rrseg.start, 55);
}
