use clap::{App, Arg, SubCommand};
use hyper::http::Response;
use hyper::{body::Bytes, body::HttpBody as _, Body, Client, Request, Uri};
use serde_json::Value;
use tokio::io::{self, AsyncWriteExt as _};

static BASE_URL: &'static str = "localhost:3000";

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// create a full url with scheme and host appending the Path and returning the
/// Uri attribute.
fn get_full_url(path: &str) -> Uri {
    return Uri::builder()
        .scheme("http")
        .authority(BASE_URL)
        .path_and_query(path)
        .build()
        .unwrap();
}

/// Call the getup API url from the provided url as the path and return the
/// response as bytes.
async fn get_request(url: &str) -> Result<Bytes> {
    let full_url = get_full_url(url);
    let client = Client::new();
    let mut resp = client.get(full_url).await?;
    let body = hyper::body::to_bytes(resp).await?;
    Ok(body)
}

async fn _request(url: &str, method: &str) -> Result<Bytes> {
    let full_url = get_full_url(url);
    // Build a request object with the full URL.
    let req = Request::builder()
        .method("POST")
        .uri(full_url)
        .body(Body::from(""))
        .expect("Failed to build request");
    let client = Client::new();
    let mut resp = client.request(req).await?;
    let body = hyper::body::to_bytes(resp).await?;
    Ok(body)
}

/// Call getupd API with a POST request.
async fn post_request(url: &str) -> Result<Bytes> {
    Ok(_request(url, "POST").await?)
}

/// Call the getupd URL and return the JSON response.
async fn get_json_response(url: &str) -> Result<Value> {
    let body = get_request(url).await?;
    let json_body: Value =
        serde_json::from_slice(&body).expect("Failed to parse json body");
    Ok(json_body)
}

async fn start_unit() {}

async fn stop_unit() {}

async fn get_unit_status() {}


/// Ask the getupd daemon to reload the unit files.
async fn reload() {
    let _ = post_request("/reload").await;
}


/// Ask the getupd daemon to gracefully shutdown.
async fn shutdown() {
    let _ = post_request("/shutdown").await;
}


/// Get all the units currently installed in the getupd daemon.
async fn get_all_units() -> Result<()> {
    let all_units = get_json_response("/units").await?;

    match all_units.get("units").unwrap() {
        Value::Array(units) => {
            for unit in units {
                pretty_print_unit(unit);
            }
        }
        _ => {
            println!("No units were found");
        }
    }

    Ok(())
}


/// Pretty print a unit object.
fn pretty_print_unit(unit: &Value) {
    println!("----");
    println!("Description: {}", unit.get("description").unwrap());
    println!("Documentation: {}", unit.get("documentation").unwrap());
    println!("State: {}", unit.get("service").unwrap().get("current_state").unwrap());
    println!(
        "RestartPolicy: {}",
        unit.get("service").unwrap().get("restart_policy").unwrap()
    );
}

#[tokio::main]
async fn main() -> Result<()> {
    let matches = App::new("getup")
        .version("0.0.1")
        .author("Abhilash Raj")
        .about("an alternate init system for GNU/Linux")
        .subcommand(SubCommand::with_name("units").about("get all units"))
        .subcommand(SubCommand::with_name("shutdown").about("Shutdown getup daemon"))
        .subcommand(SubCommand::with_name("reload").about("Reload all the unit files"))
        .get_matches();

    match matches.subcommand_name() {
        Some("units") => {
            get_all_units().await;
            ()
        }
        Some("shutdown") => {
            shutdown().await;
            ()
        }
        Some("reload") => {
            reload().await;
            ()
        }
        _ => println!("Invalid command.")
    }

    Ok(())
}
