use std::{
    collections::HashSet,
    sync::{
        LazyLock, RwLock,
        atomic::{AtomicUsize, Ordering},
    },
};

static WARNINGS: LazyLock<RwLock<HashSet<String>>> = LazyLock::new(|| RwLock::new(HashSet::new()));

static WARNING_COUNT: AtomicUsize = AtomicUsize::new(0);

pub fn append_warning(value: impl Into<String>) -> bool {
    WARNING_COUNT.fetch_add(1, Ordering::Relaxed);
    WARNINGS.write().unwrap().insert(value.into())
}

pub fn get_all_warnings() -> HashSet<String> {
    WARNINGS.read().unwrap().clone()
}

pub fn get_warning_count() -> usize {
    WARNING_COUNT.load(Ordering::Relaxed)
}
