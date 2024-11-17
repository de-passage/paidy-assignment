use common::api;
use common::errors::Result;
use common::http::{code_to_string, HttpClient, Response};
use common::routes;
use common::cli::*;
use serde;

#[derive(Debug)]
enum Action {
    Get,
    Insert,
    Delete,
}

#[derive(Debug)]
struct CLIOptions {
    target: String,
    action: Action,
    table: Option<u32>,
    orders: Vec<String>,
}

fn parse_action(action: String) -> std::result::Result<Action, CLIError> {
    match action.to_ascii_lowercase().as_str() {
        "get" => Ok(Action::Get),
        "insert" => Ok(Action::Insert),
        "delete" => Ok(Action::Delete),
        _ => Err(CLIError::InvalidParameter),
    }
}

fn parse_cli_args<I>(mut args: I) -> Result<CLIOptions>
where
    I: Iterator<Item = String>,
{
    assert!(args.next().is_some()); // Skip the program name
    let maybe_target = args
        .next()
        .ok_or(CLIError::MissingParameter("target or action"))?;

    let (target, action) = match validate_url(&maybe_target.as_str()) {
        Ok(target) => (
            target,
            args.next()
                .ok_or(CLIError::MissingParameter("action"))
                .and_then(&parse_action)?,
        ),
        Err(_) => (DEFAULT_ADDRESS, parse_action(maybe_target)?),
    };

    let table = args.next();
    if table.is_none() {
        return Ok(CLIOptions {
            target: target.to_string(),
            action,
            table: None,
            orders: Vec::new(),
        });
    }
    let table = table
        .unwrap()
        .parse::<u32>()
        .map_err(|_| CLIError::InvalidParameter)?;

    let orders = args.collect::<Vec<_>>();

    Ok(CLIOptions {
        target: target.to_string(),
        action,
        table: Some(table),
        orders,
    })
}

fn print_response<'a, Body>(response: &'a Response)
where
    Body: serde::Deserialize<'a> + std::fmt::Debug,
{
    match response.status {
        Some(code) => println!("Response Status: {} - {}", code, code_to_string(code)),
        None => println!("No status in response"),
    }
    if !response.body.is_empty() {
        let json = serde_json::from_str::<Body>(&response.body);
        match json {
            Ok(json) => println!("Response Body: {:?}", json),
            Err(e) => println!("Error parsing response body: {}\n{:?}", e, response.body),
        }
    }
}

fn main() {
    let options = parse_cli_args(std::env::args()).unwrap();

    let mut client = HttpClient::new(&options.target).unwrap();

    match options.action {
        Action::Get => {
            let table = options.table.unwrap();

            if options.orders.is_empty() {
                let response = client
                    .send("GET", &routes::order_by_id(table).as_str(), "")
                    .unwrap();
                print_response::<api::Order>(&response);
                return;
            }

            let orders = options
                .orders
                .iter()
                .map(|order| {
                    order
                        .parse::<u32>()
                        .map_err(|_| CLIError::InvalidParameter.into())
                })
                .collect::<Result<Vec<u32>>>()
                .unwrap();

            for order in orders {
                let response = client
                    .send("GET", &routes::item_by_id(table, order).as_str(), "")
                    .unwrap();
                print_response::<api::Item>(&response);
            }
        }
        Action::Insert => {
            let table = options.table.unwrap();
            let body = api::NewOrder {
                items: options.orders.clone(),
                table_number: table,
            };

            let response = client
                .send(
                    "POST",
                    routes::paths::ORDERS,
                    &serde_json::to_string(&body).unwrap().as_str(),
                )
                .unwrap();
            print_response::<api::Order>(&response);
        }
        Action::Delete => {
            let table = options.table.unwrap();

            let orders = options
                .orders
                .iter()
                .map(|order| {
                    order
                        .parse::<u32>()
                        .map_err(|_| CLIError::InvalidParameter.into())
                })
                .collect::<Result<Vec<u32>>>()
                .unwrap();

            if orders.is_empty() {
                panic!("Missing parameter 'item id'")
            }

            for item in orders {
                let response = client
                    .send("DELETE", &routes::item_by_id(table, item).as_str(), "")
                    .unwrap();
                print_response::<api::Item>(&response);
            }
        }
    }
}
