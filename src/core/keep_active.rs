use crate::config::KeepActiveConfig;
use std::thread;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::time::Duration;
use windows::Win32::System::Power::{SetThreadExecutionState, ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED};


pub fn start_keep_active(cfg: &KeepActiveConfig) -> Option<(Arc<AtomicBool>, thread::JoinHandle<()>)> {
    if !cfg.enabled {
        return None;
    }

    let ivl_secs = cfg.interval_seconds.max(15);

    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = stop.clone();
    let ivl = Duration::from_secs(ivl_secs);

    let hndl = thread::spawn(move || {
        unsafe {
            while !stop_clone.load(Ordering::Relaxed) {
                SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED);
                thread::sleep(ivl);
            }

            SetThreadExecutionState(ES_CONTINUOUS);
        }
    });

    Some((stop, hndl))
}