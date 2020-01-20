use std::fs::File;

use daemonize::Daemonize;
use hyper::rt::Future;
use hyper::Server;

use getup::api::router_service;
use getup::conf::{SETTINGS, initialize_config};
use getup::core::initialize;
use std::env;
use std::process;

fn usage(args: &Vec<String>) {
    println!("Expected 1 parameter, got {:?}", args);
    println!("\nUsage: getupd /path/all/services/dir");
}

fn main() {

    initialize_config();

    let args: Vec<String> = env::args().collect();
    let mut services_path: &str = &SETTINGS.services_path;
    if args.len() == 2 {
       services_path = &args[1];
    } else if args.len() > 2 {
        usage(&args);
        process::exit(1);
    }

    initialize(services_path);

    let stdout = File::create(&SETTINGS.stdout).unwrap();
    let stderr = File::create(&SETTINGS.stderr).unwrap();

    let daemon = Daemonize::new()
        .pid_file(&SETTINGS.pidfile)
        .chown_pid_file(true)
        .working_directory(&SETTINGS.workdir)
        .umask(0o777)
        .stdout(stdout)
        .stderr(stderr)
        .exit_action(|| println!("Exitting..."))
        .privileged_action(|| println!("Dropping privileges"));

    // This is our socket address...
    let addr = format!("0.0.0.0:{}", SETTINGS.port);

    // This is our server object.
    let server = Server::bind(&addr.parse().expect("Unable to parse host port"))
        .serve(router_service)
        .map_err(|e| eprintln!("server error: {}", e));

    println!("API Server running on {}", addr);
    // Run this server for... forever!
    hyper::rt::run(server);
    // This never reaches due to the previous line.
    match daemon.start() {
        Ok(_) => println!("Started getupd..."),
        Err(e) => eprintln!("Error, {}", e),
    }
}
