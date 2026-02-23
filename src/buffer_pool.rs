use std::sync::OnceLock;
use crossbeam_queue::ArrayQueue;

const POOL_CAPACITY: usize = 256;
const MIN_BUF_CAPACITY: usize = 1500;

static POOL: OnceLock<ArrayQueue<Vec<u8>>> = OnceLock::new();

fn pool() -> &'static ArrayQueue<Vec<u8>> {
    POOL.get_or_init(|| ArrayQueue::new(POOL_CAPACITY))
}

pub fn get(min_capacity: usize) -> Vec<u8> {
    if let Some(mut buf) = pool().pop() {
        buf.clear();
        if buf.capacity() < min_capacity {
            buf.reserve(min_capacity);
        }
        buf
    } else {
        Vec::with_capacity(min_capacity.max(MIN_BUF_CAPACITY))
    }
}

pub fn put(buf: Vec<u8>) {
    let _ = pool().push(buf);
}
