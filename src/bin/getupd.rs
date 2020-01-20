use std::fs::File;

use daemonize::Daemonize;
use hyper::rt::Future;
use hyper::Server;

use getup::units::{Unit, ALL_UNITS};
use getup::api::{router_service};
use std::env;
use std::ffi::OsStr;
use std::path::Path;
use std::process;


fn usage(args: &Vec<String>) {
    println!("Expected 1 parameter, got {:?}", args);
    println!("\nUsage: getupd /path/all/services/dir");
}


fn load_all_services(path: &str) {
    let services_path = Path::new(path);

    if !services_path.exists() {
        println!("Give {} path does not exist...", services_path.display());
        process::exit(1);
    }
    if !services_path.is_dir() {
        println!("Expected {} to a directory...", services_path.display());
        process::exit(1);
    }

    let all_services = services_path
        .read_dir()
        .expect("read_dir call failed")
        .filter(|entry| {
            entry
                .as_ref()
                .expect("Failed to check if the path is a file")
                .path()
                .is_file()
        })
        .filter(|entry| {
            entry
                .as_ref()
                .expect("Failed to check if the path has service extension")
                .path()
                .extension()
                == Some(OsStr::new("service"))
        });

    for entry in all_services {
        if let Ok(an_entry) = entry {
            println!("Loading {:?}...", an_entry);

            let unit = Unit::from_unitfile(&an_entry.path().as_path());
            ALL_UNITS.lock().expect("Failed to parse unit file").add_unit(unit);
        }
    }
}


fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        usage(&args);
        process::exit(1);
    }

    load_all_services(&args[1]);

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
