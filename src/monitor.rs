/// monitor.rs includes methods to monitor a running child process.
use std::process::{Child, ExitStatus};
use std::{thread, time};
use std::io::{self, Write};


pub fn monitor_proc(child: &mut Child) -> Option<ExitStatus> {
    let thirty_millis = time::Duration::from_millis(30);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                println!("Child proc with PID {:?} exitted with status {:?}", child.id(), status);
                return Some(status);
            },
            Ok(None) => {
                // This really means that the process hasn't exitted yet. In
                // which case, we don't do anything except sleeping for a while
                // and re-checking in some time.
                thread::sleep(thirty_millis);
                print!(".");
                io::stdout().flush().unwrap();
                continue
            },
            Err(e) => {
                println!("Failed to wait for the child process: {:?}", e);
                return None;
            }
        }
    }
}
