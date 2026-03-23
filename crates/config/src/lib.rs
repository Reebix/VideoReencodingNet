use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Config {
    #[arg(long, default_value_t = 8080)]
    pub port: u16,

    #[arg(short, long, default_value_t = String::from(""))]
    pub path: String,
}

pub fn init_config() -> Config {
    Config::parse()
}
