use clap::Parser;


#[derive(Parser, Debug)]
#[command(author, version, about, long_about=None)]
pub struct Args {
    #[arg(short, long, default_value = "spacetraders.json")]
    pub config_file: String
}