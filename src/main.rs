use std::ptr::null_mut;
use std::sync::Arc;
use std::time::Duration;

use tokio::runtime::Runtime;
use tracing::level_filters::LevelFilter;
use tracing::trace;

const LOG_LEVEL: LevelFilter = LevelFilter::TRACE;

pub struct SessionInner {}

impl SessionInner {
    pub async fn run(&self, id: usize, dur: Duration) {
        trace!("Thread ID {} enters.", id);
        tokio::time::sleep(dur).await;
        trace!("Thread ID {} exists.", id);
    }
}

pub struct BlockingSession {
    rt: Runtime,
    inner: SessionInner,
}

impl BlockingSession {
    pub fn new() -> Self {
        Self {
            rt: tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap(),
            inner: SessionInner {},
        }
    }

    pub fn run(&self, id: usize, dur: Duration) {
        self.rt.block_on(self.inner.run(id, dur));
    }
}

/// \brief Session
#[allow(non_camel_case_types)]
pub type nux_session_t = *mut BlockingSession;

pub unsafe extern "C" fn nux_session_create(sess: &mut nux_session_t) {
    let raw_sess = Arc::new(BlockingSession::new());
    *sess = Arc::into_raw(raw_sess) as *mut BlockingSession;
}

unsafe fn nux_session(sess: nux_session_t) -> &'static BlockingSession {
    sess.as_ref().unwrap()
}

#[no_mangle]
pub unsafe extern "C" fn nux_session_run(
    raw: nux_session_t,
    id: usize,
    msecs: u64,
) {
    let sess = nux_session(raw);
    sess.run(id, Duration::from_millis(msecs));
}

pub unsafe extern "C" fn nux_session_destroy(raw: nux_session_t) {
    Arc::from_raw(raw);
}

#[derive(Copy, Clone, Debug)]
struct PtrWrapper(*mut BlockingSession);

impl PtrWrapper {
    fn new(ptr: *mut BlockingSession) -> Self { PtrWrapper(ptr) }
}

// This is clearly safe because nothing can be done
// with a Useless
unsafe impl Send for PtrWrapper {}

// Likewise, nothing can be done with &Useless
unsafe impl Sync for PtrWrapper {}

fn main() {
    let _ = tracing_subscriber::fmt().with_max_level(LOG_LEVEL).try_init();

    let mut raw: nux_session_t = null_mut();
    unsafe {
        nux_session_create(&mut raw);
        let thread_num = 4;
        let sleep_ms = 500;
        let mut threads = Vec::with_capacity(thread_num);

        let ptr = PtrWrapper::new(raw);

        for id in 0..thread_num {
            threads.push(std::thread::spawn(move || {
                trace!("{:?}", ptr);
                for _ in 0..10 {
                    nux_session_run(ptr.0, id, sleep_ms);
                }
            }));
        }

        for thread in threads {
            thread.join().unwrap();
        }

        nux_session_destroy(raw);
    }
}


