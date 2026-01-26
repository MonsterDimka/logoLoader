use clap::Parser;

pub const BASE_PATH: &str = "/Users/kapustindmitri/RustroverProjects/logoLoader/";
pub const JSON_FILE_PATH: &str = "job.json";
pub const DOWNLOAD_FOLDER: &str = "Logo/Raw";
pub const UPSCALE_FOLDER: &str = "Logo/Upscale";
pub const LOG_FILE: &str = "logo.log";
pub const RESULT_FOLDER: &str = "Logo/Result";
pub const CROP_FOLDER: &str = "Logo/Crop";
pub const TEMP_JOB_FILE: &str = "job.json";
pub const DOWNLOAD: bool = true;
pub const UPSCALE: bool = true;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Config {
    /// JSON file with logos job
    #[arg(short, long, default_value_t = JSON_FILE_PATH.to_string())]
    pub job: String,

    /// PNG optimization level
    #[arg(short, long, default_value_t = BASE_PATH.to_string())]
    pub out_dir: String,

    #[arg(long, default_value_t = DOWNLOAD)]
    pub download: bool,

    /// Whether to upscale images
    #[arg(long, default_value_t = UPSCALE)]
    pub upscale: bool,
}

impl Config {
    pub fn get() -> Config {
        Config::parse()
    }
}
