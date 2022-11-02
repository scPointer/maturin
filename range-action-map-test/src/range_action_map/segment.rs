pub use super::types::*;

/// 维护一段相同权限的地址区间的线段
pub trait Segment: Sized {
    /// 删除这段区间
    fn remove(&mut self, args: ArgsType);
    /// 拆分这段区间。self 结构变成左半边，返回右半边
    fn split(&mut self, pos: CmpType, args: ArgsType) -> Self;
    /// 修改区间的属性
    fn modify(&mut self, new_flag: IdentType, args: ArgsType);

    /// 按 pos 拆分区间，只保留左半边
    fn shrink_to_left(&mut self, pos: CmpType, args: ArgsType) {
        self.split(pos, args).remove(args);
    }
    /// 按 pos 拆分区间，只保留右半边
    fn shrink_to_right(&mut self, pos: CmpType, args: ArgsType) {
        let right = self.split(pos, args);
        self.remove(args);
        *self = right;
    }
    /// 按 pos_left 和 pos_right 把区间拆成三段，**默认不检查 pos_left <= pos_right**。
    /// 然后 self 结构变成左半边区间，删除中间的区间，返回右半边区间
    fn split_and_remove_middle(&mut self, pos_left: CmpType, pos_right: CmpType, args: ArgsType) -> Self {
        let right = self.split(pos_right, args);
        self.split(pos_left, args).remove(args);
        right
    }
    /// 按 pos 拆分区间，并将左半边区间的属性修改为 new_flag。
    /// self 结构变成左半边，返回右半边
    fn modify_left(&mut self, pos: CmpType, new_flag: IdentType, args: ArgsType) -> Self {
        let right = self.split(pos, args);
        self.modify(new_flag, args);
        right
    }
    /// 按 pos 拆分区间，并将右半边区间的属性修改为 new_flag。
    /// self 结构变成左半边，返回右半边
    fn modify_right(&mut self, pos: CmpType, new_flag: IdentType, args: ArgsType) -> Self {
        let mut right = self.split(pos, args);
        right.modify(new_flag, args);
        right
    }
    /// 按 pos_left 和 pos_right 把区间拆成三段，**默认不检查 pos_left <= pos_right**，
    /// 然后修改中间一段的属性。
    /// self 结构变成左半边，返回中间和右半边
    fn modify_middle(&mut self, pos_left: CmpType, pos_right: CmpType, new_flag: IdentType, args: ArgsType) -> (Self, Self) {
        let right = self.split(pos_right, args);
        let mut middle = self.split(pos_left, args);
        middle.modify(new_flag, args);
        (middle, right)
    }
}
