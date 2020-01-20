use std::fs::File;

use daemonize::Daemonize;
use hyper::rt::Future;
use hyper::Server;

use getup::api::router_service;
use getup::core::initialize;
use std::env;
use std::process;

fn usage(args: &Vec<String>) {
    println!("Expected 1 parameter, got {:?}", args);
    println!("\nUsage: getupd /path/all/services/dir");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        usage(&args);
        process::exit(1);
    }

    initialize(&args[1]);

    let stdout = File::create("/tmp/getupd-out.txt").unwrap();
    let stderr = File::create("/tmp/getupd-err.txt").unwrap();

    let daemon = Daemonize::new()
        .pid_file("/tmp/getupd.pid")
        .chown_pid_file(true)
        .working_directory("/tmp")
        .umask(0o777)
        .stdout(stdout)
        .stderr(stderr)
        .exit_action(|| println!("Exitting..."))
        .privileged_action(|| println!("Dropping privileges"));

    // This is our socket address...
    let addr = ([127, 0, 0, 1], 3000).into();

    // This is our server object.
    let server = Server::bind(&addr)
        .serve(router_service)
        .map_err(|e| eprintln!("server error: {}", e));

    // Run this server for... forever!
    hyper::rt::run(server);

    // This never reaches due to the previous line.
    match daemon.start() {
        Ok(_) => println!("Started getupd..."),
        Err(e) => eprintln!("Error, {}", e),
    }
}
