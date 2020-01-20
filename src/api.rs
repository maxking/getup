use crate::units::{reload_server, ALL_UNITS};
/// Module that includes all handler functions for the HTTP API.
use hyper::header::CONTENT_TYPE;
use hyper::StatusCode;
use hyper::{Body, Request, Response};
use hyper_router::{Route, RouterBuilder, RouterService};
use serde_json;

/// Router service that routes requests to appropriate handler method based on
/// the regex.
///
/// /             -> Returns just a string.
/// /units        -> Returns a list of units installed.
/// /units/<unit> -> Return the details of the specific unit.
pub fn router_service() -> Result<RouterService, std::io::Error> {
    let router = RouterBuilder::new()
        .add(Route::get("/").using(root))
        .add(Route::get("/reload").using(reload))
        .add(Route::get("/units").using(get_all_units))
        .add(Route::get("/unit/.*?").using(get_a_unit))
        .add(Route::post("/unit/.*?/start").using(start_service))
        .add(Route::post("/unit/.*?/stop").using(stop_service))
        .build();

    Ok(RouterService::new(router))
}

/// Handle: /
fn root(_: Request<Body>) -> Response<Body> {
    let body = "Try GET to /units";
    Response::builder()
        .header(CONTENT_TYPE, "text/plain")
        .body(Body::from(body))
        .expect("Failed to construct the response")
}

/// Handle: /units
fn get_all_units(_: Request<Body>) -> Response<Body> {
    Response::builder()
        .header(CONTENT_TYPE, "application/json")
        // TODO: Fix the serialization of the units.
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


// TODO:
/// Handle: /reload
fn reload(_: Request<Body>) -> Response<Body> {
    reload_server();
    Response::new(Body::empty())
}


fn start_service(req: Request<Body>) -> Response<Body> {
    let mut response = Response::new(Body::empty());
    let path = req.uri().path();
    let service = path.split("/").collect::<Vec<&str>>()[2];

    if let Some(unit) = ALL_UNITS.lock().unwrap().get_by_name(service) {
        unit.service.lock().unwrap().start();
        *response.body_mut() = Body::from("OK");
    } else {
        // Nothing found with that name.
        *response.status_mut() = StatusCode::NOT_FOUND;
        *response.body_mut() = Body::from(format!("Unknown service {}", service));
    }

    response
}


fn stop_service(req: Request<Body>) -> Response<Body> {
    let mut response = Response::new(Body::empty());
    let path = req.uri().path();
    let service = path.split("/").collect::<Vec<&str>>()[2];

    if let Some(unit) = ALL_UNITS.lock().unwrap().get_by_name(service) {
        unit.service.lock().unwrap().stop();
        *response.body_mut() = Body::from("OK");
    } else {
        // Nothing found with that name.
        *response.status_mut() = StatusCode::NOT_FOUND;
        *response.body_mut() = Body::from(format!("Unknown service {}", service));
    }

    response
}
