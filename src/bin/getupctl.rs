use hyper::{Client, Uri, body::HttpBody as _};
use serde_json::{Value};
use tokio::io::{self, AsyncWriteExt as _};

use clap::{Arg, App, SubCommand};


static BASE_URL: &'static str = "http://localhost:3000";


type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;


pub fn parse_json(jsonstr: &str) -> Value {
    return serde_json::from_str(&jsonstr).unwrap();
}


async fn get_json_response(url: Uri) -> Result<Value> {
    let client = Client::new();
    let mut res = client.get(url).await?;
    let body = hyper::body::to_bytes(res).await?;
    let json_body: Value = serde_json::from_slice(&body).unwrap();
    Ok(json_body)
}


async fn start_unit() {
}


async fn stop_unit() {
}


async fn get_unit_status() {
}


async fn get_all_units() -> Result<()>{
    let all_units = get_json_response("http://localhost:3000/units".parse::<Uri>().unwrap()).await?;

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
}


#[tokio::main]
async fn main() -> Result<()> {

    let matches = App::new("getup")
        .version("0.0.1")
        .author("Abhilash Raj")
        .about("an alternate init system for GNU/Linux")
        .subcommand(SubCommand::with_name("units")
                    .about("get all units"))
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("units") {
        get_all_units().await;
    }

    Ok(())
}
