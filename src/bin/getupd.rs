use std::fs::File;

use daemonize::Daemonize;
use hyper::rt::Future;
use hyper::service::service_fn;
use hyper::{Body, Request, Response, Server};

use futures::future;
use hyper::{Method, StatusCode};

use getup::units::{AllUnits, Unit};
use serde_json;
use std::env;
use std::ffi::OsStr;
use std::path::Path;
use std::process;
use std::sync::Arc;
use std::sync::Mutex;

fn usage(args: &Vec<String>) {
    println!("Expected 1 parameter, got {:?}", args);
    println!("\nUsage: getupd /path/all/services/dir");
}

type BoxFut = Box<dyn Future<Item = Response<Body>, Error = hyper::Error> + Send>;

fn handler(req: Request<Body>, all_units: &Mutex<AllUnits>) -> BoxFut {
    let mut response = Response::new(Body::empty());

    // Pattern match on the request's METHOD and URI to decide what to do.
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => {
            *response.body_mut() = Body::from("Try GET to /services");
        }
        (&Method::GET, "/units") => {
            *response.body_mut() =
                Body::from(serde_json::to_string(&all_units).unwrap());
        }
        // Match all the paths, so we can do partial string match on them.
        (&Method::GET, path)  => {
            println!("Got a request for {:?}", path);

            // If the paths is of the form /unit/unit.service
            if path.starts_with("/unit/") {
                let service = path.split("/").collect::<Vec<&str>>()[2];

                // Lookup if there is a service by that name loaded.
                if let Some(unit) = all_units.lock().unwrap().get_by_name(service) {
                    *response.body_mut() =
                        Body::from(serde_json::to_string(&unit).unwrap());
                } else {
                    // Nothing found with that name.
                    *response.status_mut() = StatusCode::NOT_FOUND;
                }
            } else {
                // All other requests, not starting with /unit/ returns 404.
                *response.status_mut() = StatusCode::NOT_FOUND;
            }
        }
        // All other requests, not matching above patterns returns 404.
        (_) =>  *response.status_mut() = StatusCode::NOT_FOUND
    }
    Box::new(future::ok(response))
}


fn load_all_services(path: &str) -> Arc<Mutex<AllUnits>> {
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

    let all_units = Arc::new(Mutex::new(AllUnits::new()));

    for entry in all_services {
        if let Ok(an_entry) = entry {
            println!("Loading {:?}...", an_entry);

            let unit = Unit::from_unitfile(&an_entry.path().as_path());
            all_units.lock().expect("Failed to parse unit file").add_unit(unit);
        }
    }

    all_units
}


fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        usage(&args);
        process::exit(1);
    }

    let all_units = load_all_services(&args[1]);

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
