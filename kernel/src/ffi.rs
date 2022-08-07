/// 自终止对象。
///
/// 对象的值描述了终止语义。
pub trait Terminatable {
    fn is_ternimator(&self) -> bool;
}

/// 自终止切片。
///
/// 来自 C ffi 的不安全结构。
pub struct WithTerminator<T: Terminatable>(pub *const T);

/// ascii 字符串以 b'\0' 终止。
impl Terminatable for u8 {
    #[inline]
    fn is_ternimator(&self) -> bool {
        *self == b'\0'
    }
}

/// 指针以空指针终止。
impl<T> Terminatable for *const T {
    #[inline]
    fn is_ternimator(&self) -> bool {
        (*self).is_null()
    }
}

impl<T: Terminatable> WithTerminator<T> {
    /// 计算切片长度。
    #[inline]
    pub unsafe fn len(&self) -> usize {
        let mut ptr = self.0;
        while !(*ptr).is_ternimator() {
            ptr = ptr.add(1);
        }
        ptr.offset_from(self.0) as _
    }

    /// 转化为切片。
    #[inline]
    pub unsafe fn as_slice(&self) -> &'static [T] {
        core::slice::from_raw_parts(self.0, self.len())
    }
}

/// 遍历自终止切片。
impl<T: 'static + Terminatable> Iterator for WithTerminator<T> {
    type Item = &'static T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let ans = Some(unsafe { &*self.0 }).filter(|t| !t.is_ternimator());
        if ans.is_some() {
            unsafe {
                self.0 = self.0.add(1);
            }
        }
        ans
    }
}

impl WithTerminator<u8> {
    /// ascii 自终止字符串。
    #[inline]
    pub unsafe fn as_str(&self) -> &'static str {
        core::str::from_utf8(self.as_slice()).unwrap()
    }
}
