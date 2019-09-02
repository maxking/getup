use getup::{units, monitor};
/// run one is a script which reads a systems configuration path and spawns off
/// the service and keeps on monitoring it.
use std::env;
use std::process;
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

    if let Some(mut child) = unit.service.child {
        let mon_thread = thread::spawn(move || {
            monitor::monitor_proc(&mut child);
        });

        let _ = mon_thread.join();
    }
}
