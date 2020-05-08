use hyper::{body::HttpBody as _, Client, Uri};
use serde_json::Value;
use tokio::io::{self, AsyncWriteExt as _};

use clap::{App, Arg, SubCommand};

static BASE_URL: &'static str = "localhost:3000";

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub fn parse_json(jsonstr: &str) -> Value {
    return serde_json::from_str(&jsonstr).unwrap();
}

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

/// Call the URL and return the JSON response.
async fn get_json_response(url: &str) -> Result<Value> {
    let full_url = get_full_url(url);
    let client = Client::new();
    let mut res = client.get(full_url).await?;
    let body = hyper::body::to_bytes(res).await?;
    let json_body: Value = serde_json::from_slice(&body).unwrap();
    Ok(json_body)
}

async fn start_unit() {}

async fn stop_unit() {}

async fn get_unit_status() {}

async fn shutdown() {}

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
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("units") {
        get_all_units().await;
    }

    Ok(())
}
