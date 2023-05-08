mod config;

use std::collections::HashMap;
use std::fmt::format;

use std::fs::File;
use std::io::{BufReader};
use std::path::Path;
use repl_rs::{Command, Parameter, Repl, Result as ReplResult, Value};
use config::GameConfig;
use inquire::{Confirm};
use reqwest::StatusCode;
use spacetraders_sdk::apis::configuration::Configuration;
use spacetraders_sdk::models::register_request::Faction;
use spacetraders_sdk::models::RegisterRequest;
use chrono::{DateTime, Utc};
use comfy_table::Table;
use crate::config::ConfigWrapper;

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

fn read_config() -> Result<GameConfig, std::io::Error> {

    if !Path::new("spacetraders.json").exists() {
        println!("Config file does not exist");

        let confirmation = Confirm::new("Do you want to create a new config file? ")
            .with_default(true)
            .prompt();

        match confirmation {
            Ok(true) => {
                println!("Creating new config file at ./spacetraders.json");
                let default_config = GameConfig::default();
                std::fs::write("../spacetraders.json", serde_json::to_string_pretty(&default_config).unwrap()).unwrap();
                exit!()
            }
            _ => {
                exit!("The spacetraders.json config file is required")
            }
        }
    }

    let file = File::open("spacetraders.json")?;

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

fn main() -> ReplResult<()>{

    if let Ok(c) = read_config() {

        let mut config = ConfigWrapper::new(c);

        let mut repl = Repl::new(config)
            .with_name("Hello")
            .with_description("Cool demo")
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
            );

        repl.run()
    }
    else {
        exit!("Failed to read config file");
        Ok(())
    }
}
