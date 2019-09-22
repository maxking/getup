use std::fs::File;

use daemonize::Daemonize;
use hyper::{Body, Request, Response, Server};
use hyper::rt::Future;
use hyper::service::service_fn;

use futures::future;
use hyper::{Method, StatusCode};

use std::env;
use getup::units::{
    AllUnits, Unit
};
use serde_json;
use std::process;
use std::sync::Arc;
use std::sync::Mutex;


fn usage(args: &Vec<String>) {
    println!("Expected 1 parameter, got {:?}", args);
    println!("\nUsage: getupd example.service");
}


type BoxFut = Box<dyn Future<Item=Response<Body>, Error=hyper::Error> + Send>;


fn handler(req: Request<Body>, all_units: &Mutex<AllUnits>) -> BoxFut {
    let mut response = Response::new(Body::empty());

    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => {
            *response.body_mut() = Body::from("Try GET data to /services");
        },
        (&Method::GET, "/units") => {
            *response.body_mut() = Body::from(serde_json::to_string(&all_units).unwrap());
        },
        _ => {
            *response.status_mut() = StatusCode::NOT_FOUND;
        }
    }
    Box::new(future::ok(response))
}


fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        usage(&args);
        process::exit(1);
    }

    let all_units = Arc::new(Mutex::new(AllUnits::new()));

    let unit = Unit::from_unitfile(&args[1]);
    all_units.lock().unwrap().add_unit(unit);

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
        .serve(move || {
            let unit_clone = all_units.clone();
            service_fn(move |req| handler(req, &unit_clone))
        })
        .map_err(|e| eprintln!("server error: {}", e));

    // Run this server for... forever!
    hyper::rt::run(server);

    // This never reaches due to the previous line.
    match daemon.start() {
        Ok(_) => println!("Started getupd..."),
        Err(e) => eprintln!("Error, {}", e),
    }
}
