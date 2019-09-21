use crate::units::{CurrState, Service};
/// monitor.rs includes methods to monitor a running child process.
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::{thread, time};

pub fn monitor_proc(service: &Mutex<Service>, shared: &AtomicBool) {
    let thirty_millis = time::Duration::from_millis(30);
    let ten_sec = time::Duration::from_millis(10000);
    let mut unlocked_service = service.lock().unwrap();

    loop {
        if shared.load(Ordering::Relaxed) {
            // Shared is a flag for parent process to signal this process to
            // terminate.
            // service.lock().unwrap().send_term();
            thread::sleep(ten_sec);
            unlocked_service.kill();

            // Re-raise the flag just to make sure we don't enter this same
            // loop next time over the loop.
            shared.store(true, Ordering::Relaxed);
        }

        match unlocked_service.try_wait() {
            Ok(Some(status)) => {
                println!(
                    "Child proc with PID {:?} exitted with status {:?}",
                    unlocked_service.child_id(),
                    status
                );
                unlocked_service.exit_status = Some(status);
                unlocked_service.current_state = CurrState::Stopped;
                break;
            }
            Ok(None) => {
                // This really means that the process hasn't exitted yet. In
                // which case, we don't do anything except sleeping for a while
                // and re-checking in some time.
                thread::sleep(thirty_millis);
                print!(".");
                io::stdout().flush().unwrap();
                continue;
            }
            Err(e) => {
                println!("Failed to wait for the child process: {:?}", e);
            }
        }
    }
}
