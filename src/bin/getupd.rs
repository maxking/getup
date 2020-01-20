use std::fs::File;

use daemonize::Daemonize;
use hyper::rt::Future;
use hyper::Server;

use getup::api::router_service;
use getup::conf::{initialize_config, SETTINGS};
use getup::core::initialize;
use getup::signals::{Message, CHANNEL};
use getup::units::ALL_UNITS;
use std::env;
use std::process;
use std::thread;

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
        .exit_action(|| println!("Switching to background..."))
        .privileged_action(|| println!("Dropping privileges"));

    // This is our socket address...
    let addr = format!("0.0.0.0:{}", SETTINGS.port);

    // This is our server object.
    let server = Server::bind(&addr.parse().expect("Unable to parse host port"))
        .serve(router_service)
        .map_err(|e| eprintln!("server error: {}", e));

    match daemon.start() {
        Ok(_) => {
            // Run this server for... forever!

            println!("Starting up API  in a different thread");
            let api_server = thread::spawn(move || {
                hyper::rt::run(server);
            });
            println!("API Server running on {}", addr);

            let rx = &CHANNEL.1;

            loop {
                match rx.lock().unwrap().recv().unwrap() {
                    Message::Shutdown => {
                        println!("Got: Shutdown signal");
                        break;
                    }
                    Message::Start(unit_name) => {
                        println!("Got start {:?}", unit_name);
                        if let Some(unit) = ALL_UNITS.lock().unwrap().get_by_name(&unit_name) {
                            unit.service.lock().unwrap().start();
                        } else {
                            println!("Did not find a service named {}", unit_name);
                        }
                    }
                    Message::Stop(unit_name) => {
                        println!("Got stop {:?}", unit_name);
                        if let Some(unit) = ALL_UNITS.lock().unwrap().get_by_name(&unit_name) {
                            unit.service.lock().unwrap().stop();
                        } else {
                            println!("Did not find a service named {}", unit_name);
                        }
                    }
                    _ => println!("Unable to handle message")
                }

            }

            api_server.join().expect("Waiting for child process to exit clean");
        }
        Err(e) => eprintln!("Error, {}", e),
    };
}
