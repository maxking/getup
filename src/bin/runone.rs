use ctrlc;
use getup::{
    monitor,
    units::{self, RestartMethod},
};
/// run one is a script which reads a systems configuration path and spawns off
/// the service and keeps on monitoring it.
use std::env;
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

fn usage(args: &Vec<String>) {
    println!("Expected 1 parameter, got {:?}", args);
    println!("\nUsage: runone example.service");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        usage(&args);
        process::exit(1);
    }
    let mut unit = units::Unit::from_unitfile(&args[1]);
    println!("Parsed unit file at {:?}", args[1]);
    println!("{:?}", unit);
    println!(
        "Starting up the service using command: {:?}",
        unit.service.lock().unwrap().exec_start
    );
    unit.service.lock().unwrap().start();

    let shared = Arc::new(AtomicBool::new(false));
    let shared_clone = shared.clone();

    let _ = ctrlc::set_handler(move || {
        // If the user wants to exit, raise the flag to signal the running
        // thread to kill the child process.
        shared_clone.store(true, Ordering::Relaxed);
    });

    loop {
        let service_clone = unit.service.clone();
        let shared_shared_clone = shared.clone();

        let mon_thread = thread::spawn(move || {
            monitor::monitor_proc(&service_clone, &shared_shared_clone);
        });

        let _ = mon_thread.join().expect("Failed to join the threads");

        let mut unlocked_service = unit.service.lock().unwrap();
        match unlocked_service.restart_policy {
            RestartMethod::Never => break,
            RestartMethod::Always => {
                println!("Restart policy is RestartMethod::Always...");
                unlocked_service.start();
            }
            RestartMethod::OnFailure => {
                println!("Restart policy is Restart::OnFailure...");
                if unlocked_service.exit_status.unwrap().success() {
                    unlocked_service.start();
                } else {
                    println!("Exitted with exit code 0, so not going to restart.");
                    break;
                }
            }
        }
    }
    // match unit.service.lock().unwrap().restart_policy {
    //     units::RestartMethod::Never => break,
    //     units::RestartMethod::Always => {
    //         println!("Restart policy for {:?} is RestartMethod::Always, restarting it...", unit.description);
    //         unit.service.lock().unwrap().start();
    //     },
    //     units::RestartMethod::OnFailure => {
    //         if unit.service.lock().unwrap().exit_status.unwrap().code().unwrap() != 0 {
    //             println!("Restart policy is RestartMethod::OnFailure, exit code was: {:?}", unit.service.lock().unwrap().exit_status);
    //             unit.service.lock().unwrap().start();
    //         }
    //     },
    // }
}
