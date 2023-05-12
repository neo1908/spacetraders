use serde::{Deserialize, Serialize};
use spacetraders_sdk::apis::configuration::Configuration;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameConfig {
    // General Settings
    pub request_timeout_secs: i32,

    // Game Settings
    pub base_path: String,
    pub access_token: String,
    pub call_sign: String,
    pub faction: String,

    // State
    pub headquarters: String,
}

impl GameConfig {
    pub fn new() -> GameConfig
    {
        GameConfig{
            request_timeout_secs: 0,
            base_path: String::new(),
            access_token: String::new(),
            call_sign: String::new(),
            faction: String::new(),
            headquarters: String::new()
        }
    }

    pub fn save(new: &GameConfig) {
        println!("Saving config");
        std::fs::write("../spacetraders.json", serde_json::to_string_pretty(new).unwrap()).unwrap();
    }
}

impl Default for GameConfig {
    fn default() -> Self {
        GameConfig {
            request_timeout_secs: 5,
            base_path: "https://api.spacetraders.io/v2".to_owned(),
            access_token: String::new(),
            call_sign: String::new(),
            faction: String::new(),
            headquarters: String::new()
        }
    }
}

#[derive(Clone)]
pub struct ConfigWrapper {
    pub user_config: GameConfig,
    pub api_config: Configuration
}

impl ConfigWrapper {
    pub fn new(game_config: GameConfig) -> ConfigWrapper {
        let mut config_wrapper = ConfigWrapper {
            user_config: game_config,
            api_config: Configuration::new()
        };

        config_wrapper.api_config.base_path = config_wrapper.user_config.base_path.clone();
        config_wrapper.api_config.bearer_access_token = Some(config_wrapper.user_config.access_token.clone());

        config_wrapper
    }

    pub fn update(mut self) {

        self.api_config.base_path = self.user_config.base_path;
        self.api_config.bearer_access_token = Option::from(self.user_config.access_token);

    }
}