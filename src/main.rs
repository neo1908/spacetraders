mod config;
mod cli;

use std::any::Any;
use std::collections::HashMap;

use std::fs::File;
use std::io::{BufReader};
use std::path::Path;
use repl_rs::{Command, Parameter, Repl, Result as ReplResult, Value};
use config::GameConfig;
use inquire::{Confirm};
use reqwest::StatusCode;

use spacetraders_sdk::models::register_request::Faction;
use spacetraders_sdk::models::RegisterRequest;
use chrono::{DateTime, Utc};
use clap::Parser;
use comfy_table::Table;
use macros::answer;

use crate::cli::Args;
use crate::config::ConfigWrapper;

// fn handle_error(msg: String, error: Box<dyn std::error::Error>) -> ReplResult<Option<String>> {
//     Ok(Some(format!(msg, error)))
// }

fn check_server(_args: HashMap<String, Value>, _context: &mut ConfigWrapper) -> ReplResult<Option<String>> {
    let res = reqwest::blocking::get("https://api.spacetraders.io/v2/").unwrap();

    let status = match res.status() {
        StatusCode::OK => {
            "Server is available".to_string()
        }
         def=> format!("Server returned {} \n {}", def, res.text().unwrap() )
    };

    Ok(Some(status))
}

fn register(args: HashMap<String, Value>, context: &mut ConfigWrapper) -> ReplResult<Option<String>> {
    let call_sign = args.get("callsign").unwrap();

    let req = RegisterRequest::new(Faction::Cosmic, call_sign.to_string());

    match spacetraders_sdk::apis::default_api::register(&context.api_config, Some(req)) {
        Ok(resp) => {
            context.user_config.access_token = resp.data.token.clone();
            context.user_config.call_sign = call_sign.to_string();
            context.user_config.faction = resp.data.faction.name;

            context.clone().update();

            Ok(Some(format!("Successfully registered. Got token ( this will be saved on exit ) {}", resp.data.token)))
        },
        Err(e) => {
            Ok(Some(format!("Failed to register player {}", e)))
        }
    }
}

fn exit(_args: HashMap<String, Value>, context: &mut ConfigWrapper) -> ReplResult<Option<String>> {
    GameConfig::save(&context.user_config);
    std::process::exit(0);
}

macro_rules! exit {
    ($a: expr) => {
        _exit(format!("{}\n ... Exiting", $a))
    };
    () => {
        _exit("... Exiting".to_owned())
    }
}

fn _exit(msg: String) {
    println!("{}", msg);
    exit(HashMap::new(), &mut ConfigWrapper::new(GameConfig::new()))
        .expect("").expect("");
}

fn read_config(config_file: String) -> Result<GameConfig, std::io::Error> {

    let config_path = Path::new(&config_file);

    if !config_path.exists() {
        println!("Config file does not exist");

        let confirmation = Confirm::new("Do you want to create a new config file? ")
            .with_default(true)
            .prompt();

        match confirmation {
            Ok(true) => {
                println!("Creating new config file at {}", config_file);
                let default_config = GameConfig::default();
                std::fs::write(&config_file, serde_json::to_string_pretty(&default_config).unwrap()).unwrap();
                exit!()
            }
            _ => {
                exit!("The config file is required")
            }
        }
    }

    let file = File::open(config_path)?;

    let reader = BufReader::new(file);

    match serde_json::from_reader(reader) {
        Ok(c) => Ok(c),
        Err(e) => {
            Err(e.into())
        }
    }
}

fn new_config(_args: HashMap<String, Value>, context: &mut ConfigWrapper) -> ReplResult<Option<String>> {

    let confirmation = Confirm::new("Are you sure you want to create a new config? (Existing config will be backed up) ")
        .with_default(false)
        .prompt();

    match confirmation {
        Ok(true) => {
            // Save the current config
            GameConfig::save(&context.user_config);
            let now: DateTime<Utc> = Utc::now();
            let formatted_now = now.format("%Y%m%d%H%M%S");
            std::fs::rename("spacetraders.json", format!("spacetraders.json.{}", formatted_now)).unwrap();
            GameConfig::save(&GameConfig::default());
            Ok(Some("Success".to_string()))
        }
        Ok(false) => {
            Ok(Some("Existing config unchanged".to_string()))
        }
        Err(e) => Ok(Some(format!("Failed to read prompt {}", e)))
    }

}

fn save_config(_args: HashMap<String, Value>, context: &mut ConfigWrapper) -> ReplResult<Option<String>> {
    GameConfig::save(&context.user_config);

    Ok(Some("Saved Config".to_string()))
}

fn get_agent(_args: HashMap<String, Value>, context: &mut ConfigWrapper) -> ReplResult<Option<String>> {

    match spacetraders_sdk::apis::agents_api::get_my_agent(&context.api_config) {
        Ok(resp) => {

            let mut table = Table::new();
            table.set_header(vec!["Account ID", "Symbol", "Headquarters", "Credits"]);
            table.add_row(vec![resp.data.account_id, resp.data.symbol, resp.data.headquarters, resp.data.credits.to_string()] );
            Ok(Some(table.to_string()))
        },
        Err(e) => {
            Ok(Some(format!("Failed to get agent data {}", e)))
        }
    }
}

fn dump_config(_args: HashMap<String, Value>, context: &mut ConfigWrapper) -> ReplResult<Option<String>> {
    Ok(Some(serde_json::to_string_pretty(&context.user_config).unwrap()))
}

fn get_ship_nav(args: HashMap<String, Value>, context: &mut ConfigWrapper) -> ReplResult<Option<String>> {

    let ship_symbol = args.get("symbol").unwrap();
    match spacetraders_sdk::apis::fleet_api::get_ship_nav(&context.api_config, ship_symbol.to_string().as_str())
    {
        Ok(resp) => {
            let ship = resp.data;
            let mut nav_table = Table::new();
            nav_table.set_header(vec!["System Symbol", "Waypoint Symbol", "From", "To", "Arrival", "Status", "Flight Mode"]);
            nav_table.add_row(vec![ship.system_symbol, ship.waypoint_symbol, format!("{}//{}", ship.route.departure.system_symbol, ship.route.departure.symbol),
                                   format!("{}//{}", ship.route.destination.system_symbol, ship.route.destination.symbol),
                                    ship.route.arrival,
                                   ship.status.to_string(), ship.flight_mode.to_string()]);

            Ok(Some(nav_table.to_string()))
        },
        Err(e) => {
            Ok(Some(format!("Failed to get nav status of {} - {}", ship_symbol, e)))
        }
    }

}
fn show_ships(_args: HashMap<String, Value>, context: &mut ConfigWrapper) -> ReplResult<Option<String>> {

    match spacetraders_sdk::apis::fleet_api::get_my_ships(&context.api_config, None, None) {
        Ok(resp) => {
            let mut table = Table::new();
            table.set_header(vec!["Symbol", "Registration", "Crew", "Frame", "Reactor", "Engine", "Modules", "Mounts", "Cargo", "Fuel"]);
            for ship in resp.data {
                table.add_row(vec![ship.symbol, ship.registration.name,  format!("{}/{}",  ship.crew.current, ship.crew.capacity),
                                   ship.frame.name, ship.reactor.name, ship.engine.name, ship.modules.len().to_string(), ship.mounts.len().to_string(),
                                   format!("{}/{}", ship.cargo.units, ship.cargo.capacity), format!("{}/{}", ship.fuel.current, ship.fuel.capacity)]);
            }
            Ok(Some(table.to_string()))
        },
        Err(e) => {
            Ok(Some(format!("Failed to get ships {}", e)))
        }
    }
}

fn show_contracts(_args: HashMap<String, Value>, context: &mut ConfigWrapper) -> ReplResult<Option<String>> {
    match spacetraders_sdk::apis::contracts_api::get_contracts(&context.api_config, None, None) {
        Ok(resp) => {
            let contracts = resp.data;
            let mut table = Table::new();
            table.set_header(vec!["ID", "Faction", "Type", "Accepted", "Fulfilled", "Expiration"]);
            for contract in contracts {
                let ctype = contract.r#type.type_id();
                table.add_row(vec![contract.id, contract.faction_symbol, "".to_string(), contract.accepted.to_string(), contract.fulfilled.to_string(), contract.expiration]);
            }

            Ok(Some(table.to_string()))
        },
        Err(e) => Ok(Some(format!("Failed to get contracts - {}", e)))
    }
}

fn show_contract(args: HashMap<String, Value>, context: &mut ConfigWrapper) -> ReplResult<Option<String>> {
    let contract = args.get("contract").unwrap();

    match spacetraders_sdk::apis::contracts_api::get_contract(&context.api_config, contract.to_string().as_str()) {
        Ok(resp) => {
            let contract = resp.data;

            let deadline = contract.terms.deadline;
            let on_accept = contract.terms.payment.on_accepted;
            let on_complete = contract.terms.payment.on_fulfilled;

            let description = format!("Deliver the below goods by {deadline}\n{on_accept} credits on acceptance, {on_complete} on completion");

            let mut table = Table::new();

            if let Some(goods) = contract.terms.deliver {
                table.set_header(vec!["Goods", "Destination", "Required", "Fulfilled"]);

                for good in goods {
                    table.add_row(vec![good.trade_symbol, good.destination_symbol, good.units_required.to_string(), good.units_fulfilled.to_string()]);
                }
            }

            let res = format!("{description}\n{table}");

            Ok(Some(res))
        },
        Err(e) => Ok(Some(format!("Failed to get contract details {}", e)))
    }
}

fn accept_contract(args: HashMap<String, Value>, context: &mut ConfigWrapper) -> ReplResult<Option<String>> {
    let contract = args.get("contract").unwrap();

    match spacetraders_sdk::apis::contracts_api::accept_contract(&context.api_config, contract.to_string().as_str()) {
        Ok(_) => {
            Ok(Some("Contract accepted".to_string()))
        },
        Err(e) => Ok(Some(format!("Failed to accept contract {}", e)))
    }
}

fn get_waypoints(args: HashMap<String, Value>, context: &mut ConfigWrapper) -> ReplResult<Option<String>> {

    let system = args.get("system").unwrap();


    match spacetraders_sdk::apis::systems_api::get_system_waypoints(&context.api_config, system.to_string().as_str(), None, None) {
        Ok(resp) => {

            let mut table = Table::new();
            table.set_header(vec!["Symbol", "Type", "Traits", "Orbitals"]);

            for waypoint in resp.data {

                let mut orbitals_str = String::new();
                waypoint.orbitals.iter().map(|o| o.symbol.clone()).for_each(|o| orbitals_str.push_str(( o.as_str().to_owned() + ",\n" ).as_str() ));

                let mut traits_str = String::new();
                waypoint.traits.iter().map(|o| o.name.clone()).for_each(|o| traits_str.push_str(( o + ",\n" ).as_str() ));

                table.add_row(vec![waypoint.symbol, waypoint.r#type.to_string(), traits_str, orbitals_str]);
            }

            Ok(Some(table.to_string()))

        },
        Err(e) => {
            Ok(Some(format!("Failed to get system waypoints {e}")))
        }
    }
}

fn show_available_ships(args: HashMap<String, Value>, context: &mut ConfigWrapper) -> ReplResult<Option<String>> {
    Ok(Some(String::new()))
}

fn main() -> ReplResult<()>{

    let args = Args::parse();

    if let Ok(c) = read_config(args.config_file) {

        let config = ConfigWrapper::new(c);

        let mut repl = Repl::new(config)
            .with_name("Spacetraders")
            .with_description("REPL CLI for spacetraders")
            .use_completion(true)
            .add_command(
                Command::new("check_server", check_server)
                    .with_help("Check the server status")
            )
            .add_command(
                Command::new("exit", exit)
                    .with_help("Exit")
            )
            .add_command(
                Command::new("register", register)
                    .with_help("Register as a new user. The default faction is \"Comsic\", see the docs for all options")
                    .with_parameter(Parameter::new("callsign").set_required(true)?)?
                    .with_parameter(Parameter::new("faction").set_required(false)?.set_default("Cosmic")?)?
            ).add_command(
            Command::new("new_config", new_config)
                .with_help("Backup the existing config and create a new one")
        ).add_command(
            Command::new("save_config", save_config)
                .with_help("Save your config")
        )
            .add_command(Command::new("dump_config", dump_config)
                             .with_help("Show current config")
            ).add_command(
            Command::new("get_agent", get_agent)
                .with_help("Show the current agent status")
        )
            .add_command(
                Command::new("show_ships", show_ships)
                    .with_help("Show all ships")
            ).add_command(
            Command::new("show_ship_nav", get_ship_nav)
                .with_parameter(Parameter::new("symbol").set_required(true)?)?
                .with_help("Get the navigation for a given ship")
        )
            .add_command(Command::new("show_contracts", show_contracts)
                             .with_help("Show available contracts"))
            .add_command(Command::new("show_contract", show_contract)
                             .with_parameter(Parameter::new("contract").set_required(true)?)?
            )
            .add_command(Command::new("accept_contract", accept_contract)
                .with_help("Accept a contract")
                .with_parameter(Parameter::new("contract").set_required(true)?)?)
            .add_command(Command::new("system_waypoints", get_waypoints)
            .with_help("Show system waypoints")
            .with_parameter(Parameter::new("system").set_required(false)?)?);

        repl.run()
    }
    else {
        exit!("Failed to read config file");
        Ok(())
    }
}
