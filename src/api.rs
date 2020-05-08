use crate::signals::{signal_daemon, Message};
use crate::units::{reload_server, ALL_UNITS};
/// Module that includes all handler functions for the HTTP API.
use hyper::header::CONTENT_TYPE;
use hyper::StatusCode;
use hyper::{Body, Method, Request, Response};
use serde_json;

/// Router service that routes requests to appropriate handler method based on
/// the regex.
///
/// /             -> Returns just a string.
/// /units        -> Returns a list of units installed.
/// /units/<unit> -> Return the details of the specific unit.
// pub fn router_service() -> Result<RouterService, std::io::Error> {
//     let router = RouterBuilder::new()
//         .add(Route::get("/").using(root))
//         .add(Route::post("/shutdown").using(shutdown))
//         .add(Route::post("/reload").using(reload))
//         .add(Route::get("/units").using(get_all_units))
//         .add(Route::post(r"/unit/.*?/start").using(start_service))
//         .add(Route::post(r"/unit/.*?/stop").using(stop_service))
//         .add(Route::get(r"/unit/.*?").using(get_a_unit))
//         .build();

//     Ok(RouterService::new(router))
// }

pub async fn router(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => Ok(root(req)),
        (&Method::POST, "/shutdown") => Ok(shutdown(req)),
        (&Method::POST, "/reload") => Ok(reload(req)),
        (&Method::GET, "/units") => Ok(get_all_units(req)),
        _ => Ok(root(req)),
    }
}

/// Handle: /
fn root(_: Request<Body>) -> Response<Body> {
    let body = "Try GET to /units";
    Response::builder()
        .header(CONTENT_TYPE, "text/plain")
        .body(Body::from(body))
        .expect("Failed to construct the response")
}

/// Handle: /shutdown
fn shutdown(_: Request<Body>) -> Response<Body> {
    signal_daemon(Message::Shutdown);

    Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(""))
        .expect("Failed to construct the response")
}

/// Handle: /units
fn get_all_units(_: Request<Body>) -> Response<Body> {
    Response::builder()
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(ALL_UNITS.lock().unwrap().to_string()))
        .expect("Failed to construct the response")
}

/// Handle: /units/example.service
fn get_a_unit(req: Request<Body>) -> Response<Body> {
    let mut response = Response::new(Body::empty());
    let path = req.uri().path();
    let service = path.split("/").collect::<Vec<&str>>()[2];

    if let Some(unit) = ALL_UNITS.lock().unwrap().get_by_name(service) {
        *response.body_mut() = Body::from(serde_json::to_string(&unit).unwrap());
    } else {
        // Nothing found with that name.
        *response.status_mut() = StatusCode::NOT_FOUND;
    }

    response
}

// TODO: Implement this.
/// Handle: /reload
fn reload(_: Request<Body>) -> Response<Body> {
    reload_server();
    Response::new(Body::empty())
}

/// Handle: /unit/example.service/start
fn start_service(req: Request<Body>) -> Response<Body> {
    let mut response = Response::new(Body::empty());
    let path = req.uri().path();
    let service = path.split("/").collect::<Vec<&str>>()[2];

    if let Some(_) = ALL_UNITS.lock().unwrap().get_by_name(service) {
        signal_daemon(Message::Start(service.to_string()));
        *response.body_mut() = Body::from("OK");
    } else {
        // Nothing found with that name.
        *response.status_mut() = StatusCode::NOT_FOUND;
        *response.body_mut() = Body::from(format!("Unknown service {}", service));
    }

    response
}

/// Handle: /unit/example.service/stop
fn stop_service(req: Request<Body>) -> Response<Body> {
    let mut response = Response::new(Body::empty());
    let path = req.uri().path();
    let service = path.split("/").collect::<Vec<&str>>()[2];

    if let Some(_) = ALL_UNITS.lock().unwrap().get_by_name(service) {
        signal_daemon(Message::Stop(service.to_string()));
        *response.body_mut() = Body::from("OK");
    } else {
        // Nothing found with that name.
        *response.status_mut() = StatusCode::NOT_FOUND;
        *response.body_mut() = Body::from(format!("Unknown service {}", service));
    }

    response
}
