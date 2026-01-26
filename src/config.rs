use clap::Parser;
use std::path::{Path, PathBuf};

pub const JSON_FILE_PATH: &str = "job.json";
pub const DOWNLOAD_FOLDER: &str = "Logo/Raw";
pub const UPSCALE_FOLDER: &str = "Logo/Upscale";
pub const LOG_FILE: &str = "logo.log";
pub const RESULT_FOLDER: &str = "Logo/Result";
pub const CROP_FOLDER: &str = "Logo/Crop";
pub const TEMP_JOB_FILE: &str = "job.json";
pub const DOWNLOAD: bool = true;
pub const UPSCALE: bool = true;

// Пути по умолчанию для Upscayl (macOS)
pub const DEFAULT_UPSCALER_PROG: &str = "/Applications/Upscayl.app/Contents/Resources/bin/upscayl-bin";
pub const DEFAULT_MODEL_PATH: &str = "/Applications/Upscayl.app/Contents/Resources/models";
pub const DEFAULT_MODEL_NAME: &str = "upscayl-standard-4x";

#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
pub struct Config {
    /// JSON file with logos job
    #[arg(short, long, default_value_t = JSON_FILE_PATH.to_string())]
    pub job: String,

    /// Output directory for all generated files
    #[arg(short, long, default_value_t = String::from("."))]
    pub out_dir: String,

    #[arg(long, default_value_t = DOWNLOAD)]
    pub download: bool,

    /// Whether to upscale images
    #[arg(long, default_value_t = UPSCALE)]
    pub upscale: bool,

    /// Path to upscayl binary
    #[arg(long, default_value_t = String::from(DEFAULT_UPSCALER_PROG))]
    pub upscayl_bin: String,

    /// Path to upscayl models directory
    #[arg(long, default_value_t = String::from(DEFAULT_MODEL_PATH))]
    pub upscayl_models: String,

    /// Upscayl model name
    #[arg(long, default_value_t = String::from(DEFAULT_MODEL_NAME))]
    pub upscayl_model: String,
}

impl Config {
    pub fn get() -> Config {
        Config::parse()
    }

    /// Получить полный путь к директории загрузки
    pub fn download_folder(&self) -> PathBuf {
        Path::new(&self.out_dir).join(DOWNLOAD_FOLDER)
    }

    /// Получить полный путь к директории увеличенных изображений
    pub fn upscale_folder(&self) -> PathBuf {
        Path::new(&self.out_dir).join(UPSCALE_FOLDER)
    }

    /// Получить полный путь к директории результатов
    pub fn result_folder(&self) -> PathBuf {
        Path::new(&self.out_dir).join(RESULT_FOLDER)
    }

    /// Получить полный путь к директории обрезанных изображений
    pub fn crop_folder(&self) -> PathBuf {
        Path::new(&self.out_dir).join(CROP_FOLDER)
    }

    /// Получить полный путь к файлу лога
    pub fn log_file(&self) -> PathBuf {
        Path::new(&self.out_dir).join(LOG_FILE)
    }

    /// Получить полный путь к временному файлу заданий
    pub fn temp_job_file(&self) -> PathBuf {
        Path::new(&self.out_dir).join(TEMP_JOB_FILE)
    }

    /// Получить список всех директорий для создания
    pub fn all_folders(&self) -> Vec<PathBuf> {
        vec![
            self.download_folder(),
            self.upscale_folder(),
            self.result_folder(),
            self.crop_folder(),
        ]
    }
}
