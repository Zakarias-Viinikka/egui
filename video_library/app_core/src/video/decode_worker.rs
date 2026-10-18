use crate::error::{AppError, AppResult};
use crate::video::decoder::VideoDecoder;
use crate::video::frame::Frame;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

struct Shared {
    pending: Option<(u64, Duration)>,
    shutdown: bool,
}

pub struct DecodeWorker {
    shared: Arc<(Mutex<Shared>, Condvar)>,
    results: Receiver<(u64, AppResult<Frame>)>,
    handle: Option<JoinHandle<()>>,
}

impl DecodeWorker {
    pub fn spawn(source: PathBuf, proxy: PathBuf) -> AppResult<(Self, Duration, f64)> {
        let shared = Arc::new((
            Mutex::new(Shared { pending: None, shutdown: false }),
            Condvar::new(),
        ));
        let shared_worker = shared.clone();
        let (res_tx, res_rx) = channel();
        let (init_tx, init_rx) = channel();

        let handle = std::thread::spawn(move || {
            let mut decoder = match VideoDecoder::open_proxy(&source, &proxy) {
                Ok(d) => d,
                Err(e) => {
                    let _ = init_tx.send(Err(e));
                    return;
                }
            };
            let _ = init_tx.send(Ok((decoder.duration(), decoder.fps())));

            loop {
                let (generation, ts) = {
                    let (lock, cvar) = &*shared_worker;
                    let mut s = lock.lock().unwrap_or_else(|p| p.into_inner());
                    while s.pending.is_none() && !s.shutdown {
                        s = cvar.wait(s).unwrap_or_else(|p| p.into_inner());
                    }
                    if s.shutdown {
                        return;
                    }
                    match s.pending.take() {
                        Some(req) => req,
                        None => return,
                    }
                };
                let result = decoder.frame_at(ts);
                if res_tx.send((generation, result)).is_err() {
                    return;
                }
            }
        });

        let (duration, fps) = init_rx
            .recv()
            .map_err(|e| AppError::new(format!("{e}"), "decode_worker::spawn", file!()))??;

        Ok((
            Self { shared, results: res_rx, handle: Some(handle) },
            duration,
            fps,
        ))
    }

    pub fn request(&self, generation: u64, ts: Duration) {
        let (lock, cvar) = &*self.shared;
        let mut s = lock.lock().unwrap_or_else(|p| p.into_inner());
        s.pending = Some((generation, ts));
        cvar.notify_one();
    }

    pub fn try_recv(&self) -> Option<(u64, AppResult<Frame>)> {
        self.results.try_recv().ok()
    }
}

impl Drop for DecodeWorker {
    fn drop(&mut self) {
        {
            let (lock, cvar) = &*self.shared;
            let mut s = lock.lock().unwrap_or_else(|p| p.into_inner());
            s.shutdown = true;
            cvar.notify_one();
        }
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}
