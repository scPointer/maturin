//! 一张全局的表，从 tid 映射到对应的 signals 数组

use super::SignalReceivers;
use alloc::{collections::BTreeMap, sync::Arc};
use lock::Mutex;

/// 从 tid 获取信号相关信息
static TID2SIGNALS: Mutex<BTreeMap<usize, Arc<Mutex<SignalReceivers>>>> =
    Mutex::new(BTreeMap::new());

/// 所有线程初始化时均需要加入表
pub fn global_register_signals(tid: usize, signals: Arc<Mutex<SignalReceivers>>) {
    TID2SIGNALS.lock().insert(tid, signals).take();
}

/// 所有线程退出时均需要从表中删除
pub fn global_logoff_signals(tid: usize) {
    TID2SIGNALS.lock().remove(&tid).take();
}

/// 获取信号量。这个函数会复制一个 Arc，不会影响表中的信号本身
pub fn get_signals_from_tid(tid: usize) -> Option<Arc<Mutex<SignalReceivers>>> {
    TID2SIGNALS.lock().get(&tid).map(|s| s.clone())
}
