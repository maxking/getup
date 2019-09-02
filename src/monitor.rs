use std::borrow::BorrowMut;
use std::io::{self, Write};
/// monitor.rs includes methods to monitor a running child process.
use std::process::{Child, ExitStatus};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{thread, time};

use crate::units::Service;

pub fn monitor_proc(service: &mut Service, shared: &AtomicBool) -> Option<ExitStatus> {
    let thirty_millis = time::Duration::from_millis(30);
    let ten_sec = time::Duration::from_millis(10000);

    loop {
        if shared.load(Ordering::Relaxed) {
            // Shared is a flag for parent process to signal this process to
            // terminate.
            println!("Killing the child process.");
            service.send_term();
            thread::sleep(ten_sec);
            service.kill();
        }

        match service.try_wait() {
            Ok(Some(status)) => {
                println!(
                    "Child proc with PID {:?} exitted with status {:?}",
                    service.child_id(),
                    status
                );
                return Some(status);
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
                return None;
            }
        }
    }
}
