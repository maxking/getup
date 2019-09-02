use ctrlc;
use getup::{monitor, units};
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
        unit.service.exec_start
    );
    unit.service.start();

    let shared = Arc::new(AtomicBool::new(false));
    let shared_clone = shared.clone();

    let _ = ctrlc::set_handler(move || {
        // If the user wants to exit, raise the flag to signal the running
        // thread to kill the child process.
        shared.store(true, Ordering::Relaxed);
    });

    let mon_thread = thread::spawn(move || {
        monitor::monitor_proc(&mut unit.service, &shared_clone);
    });

    let _ = mon_thread.join();
}
