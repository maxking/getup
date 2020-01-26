use std::fs::File;

use daemonize::Daemonize;
use futures::channel::oneshot;
use hyper::Server;
use hyper::service::{make_service_fn, service_fn};

use getup::api::{router};
use getup::conf::{initialize_config, SETTINGS};
use getup::core::initialize;
use getup::signals::{Message, CHANNEL};
use getup::units::ALL_UNITS;
use std::env;
use std::process;
use std::thread;

#[macro_use]
extern crate log;
extern crate pretty_env_logger;

fn usage(args: &Vec<String>) {
    println!("Expected 1 parameter, got {:?}", args);
    println!("\nUsage: getupd /path/all/services/
dir");
}


#[tokio::main]
async fn run(rx: oneshot::Receiver<()>) {

    // This is our socket address...
    let addr = format!("0.0.0.0:{}", SETTINGS.port);

    let service = make_service_fn(|_| async { Ok::<_, hyper::Error>(service_fn(router)) });
    info!("API Server running on {}", addr);
    // This is our server object.
    let server = Server::bind(&addr.parse().expect("Unable to parse host port"))
        .serve(service)
        .with_graceful_shutdown(async move {
            rx.await.ok();
        });

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}


fn main() {
    initialize_config();

    pretty_env_logger::formatted_builder()
        .parse_filters("getupd=trace")
        .init();

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
        .exit_action(|| info!("Switching to background..."))
        .privileged_action(|| info!("Dropping privileges"));

    // Create a channel to signal Hyper to shutdown when we receive the signal
    // from the Web API.
    let (tx, rx) = oneshot::channel::<()>();
 
    match daemon.start() {
        Ok(_) => {
            // Run this server for... forever!

            info!("Starting up API  in a different thread");
            let api_server = thread::spawn(move || {
                run(rx)
            });

            let rx = &CHANNEL.1;

            loop {
                match rx.lock().unwrap().recv().unwrap() {
                    Message::Shutdown => {
                        info!("Got: Shutdown signal");
                        break;
                    }
                    Message::Start(unit_name) => {
                        info!("Got start {:?}", unit_name);
                        if let Some(unit) =
                            ALL_UNITS.lock().unwrap().get_by_name(&unit_name)
                        {
                            unit.service.lock().unwrap().start();
                        } else {
                            error!("Did not find a service named {}", unit_name);
                        }
                    }
                    Message::Stop(unit_name) => {
                        info!("Got stop {:?}", unit_name);
                        if let Some(unit) =
                            ALL_UNITS.lock().unwrap().get_by_name(&unit_name)
                        {
                            unit.service.lock().unwrap().stop();
                        } else {
                            error!("Did not find a service named {}", unit_name);
                        }
                    }
                    _ => error!("Unable to handle message"),
                }
            }

            let _ = tx.send(());
            info!("Waiting for API Server to exit!");
            api_server.join().expect("Waiting for child process to exit clean");
        }
        Err(e) => eprintln!("Error, {}", e),
    };
}
