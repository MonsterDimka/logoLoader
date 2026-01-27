use clap::Parser;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const CONFIG_FILE_NAME: &str = "logo_loader.toml";
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

/// Структура для конфигурационного файла
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ConfigFile {
    pub job: Option<String>,
    pub out_dir: Option<String>,
    pub download: Option<bool>,
    pub upscale: Option<bool>,
    pub upscayl: Option<UpscaylConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UpscaylConfig {
    pub bin: Option<String>,
    pub models: Option<String>,
    pub model: Option<String>,
}

#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
pub struct Config {
    /// Path to configuration file (TOML format)
    #[arg(short = 'c', long = "config")]
    pub config_file: Option<String>,

    /// JSON file with logos job
    #[arg(short, long)]
    pub job: Option<String>,

    /// Output directory for all generated files
    #[arg(short, long)]
    pub out_dir: Option<String>,

    #[arg(long)]
    pub download: Option<bool>,

    /// Whether to upscale images
    #[arg(long)]
    pub upscale: Option<bool>,

    /// Path to upscayl binary
    #[arg(long)]
    pub upscayl_bin: Option<String>,

    /// Path to upscayl models directory
    #[arg(long)]
    pub upscayl_models: Option<String>,

    /// Upscayl model name
    #[arg(long)]
    pub upscayl_model: Option<String>,
}

impl Config {
    pub fn get() -> Config {
        let cli_config = Config::parse();
        cli_config.load_from_file()
    }

    /// Найти путь к конфигурационному файлу
    fn find_config_path(&self) -> PathBuf {
        // Если путь указан явно, используем его
        if let Some(path) = &self.config_file {
            return PathBuf::from(path);
        }

        // Ищем в текущей директории
        let current = PathBuf::from(".").join(CONFIG_FILE_NAME);
        if current.exists() {
            return current;
        }

        // Ищем в домашней директории
        if let Some(home) = std::env::var_os("HOME") {
            let home_config = PathBuf::from(home).join(format!(".{}", CONFIG_FILE_NAME));
            if home_config.exists() {
                return home_config;
            }
        }

        // Возвращаем путь в текущей директории (даже если не существует)
        current
    }

    /// Загрузить конфигурацию из файла
    fn load_file_config(path: &Path) -> Option<ConfigFile> {
        if !path.exists() {
            return None;
        }

        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("Не удалось прочитать конфигурационный файл {}: {}", path.display(), e);
                return None;
            }
        };

        match toml::from_str(&content) {
            Ok(config) => {
                println!("Конфигурация загружена из файла: {}", path.display());
                Some(config)
            }
            Err(e) => {
                eprintln!("Ошибка парсинга конфигурационного файла {}: {}", path.display(), e);
                None
            }
        }
    }

    /// Загрузить конфигурацию из файла и объединить с CLI аргументами
    /// CLI аргументы имеют приоритет над значениями из файла
    fn load_from_file(self) -> Config {
        let config_path = self.find_config_path();
        let file_config = Self::load_file_config(&config_path);
        let upscayl = file_config.as_ref().and_then(|f| f.upscayl.as_ref());
        
        Config {
            config_file: self.config_file,
            // CLI -> File -> Default (handled by getter methods)
            job: self.job
                .or(file_config.as_ref().and_then(|f| f.job.clone())),
            out_dir: self.out_dir
                .or(file_config.as_ref().and_then(|f| f.out_dir.clone())),
            download: self.download
                .or(file_config.as_ref().and_then(|f| f.download)),
            upscale: self.upscale
                .or(file_config.as_ref().and_then(|f| f.upscale)),
            upscayl_bin: self.upscayl_bin
                .or(upscayl.and_then(|u| u.bin.clone())),
            upscayl_models: self.upscayl_models
                .or(upscayl.and_then(|u| u.models.clone())),
            upscayl_model: self.upscayl_model
                .or(upscayl.and_then(|u| u.model.clone())),
        }
    }

    /// Получить значение job (гарантированно Some после load_from_file)
    pub fn job(&self) -> &str {
        self.job.as_deref().unwrap_or(JSON_FILE_PATH)
    }

    /// Получить значение out_dir (гарантированно Some после load_from_file)
    pub fn out_dir(&self) -> &str {
        self.out_dir.as_deref().unwrap_or(".")
    }

    /// Получить значение download (гарантированно Some после load_from_file)
    pub fn download(&self) -> bool {
        self.download.unwrap_or(DOWNLOAD)
    }

    /// Получить значение upscale (гарантированно Some после load_from_file)
    pub fn upscale(&self) -> bool {
        self.upscale.unwrap_or(UPSCALE)
    }

    /// Получить значение upscayl_bin (гарантированно Some после load_from_file)
    pub fn upscayl_bin(&self) -> &str {
        self.upscayl_bin.as_deref().unwrap_or(DEFAULT_UPSCALER_PROG)
    }

    /// Получить значение upscayl_models (гарантированно Some после load_from_file)
    pub fn upscayl_models(&self) -> &str {
        self.upscayl_models.as_deref().unwrap_or(DEFAULT_MODEL_PATH)
    }

    /// Получить значение upscayl_model (гарантированно Some после load_from_file)
    pub fn upscayl_model(&self) -> &str {
        self.upscayl_model.as_deref().unwrap_or(DEFAULT_MODEL_NAME)
    }

    /// Получить полный путь к директории загрузки
    pub fn download_folder(&self) -> PathBuf {
        Path::new(self.out_dir()).join(DOWNLOAD_FOLDER)
    }

    /// Получить полный путь к директории увеличенных изображений
    pub fn upscale_folder(&self) -> PathBuf {
        Path::new(self.out_dir()).join(UPSCALE_FOLDER)
    }

    /// Получить полный путь к директории результатов
    pub fn result_folder(&self) -> PathBuf {
        Path::new(self.out_dir()).join(RESULT_FOLDER)
    }

    /// Получить полный путь к директории обрезанных изображений
    pub fn crop_folder(&self) -> PathBuf {
        Path::new(self.out_dir()).join(CROP_FOLDER)
    }

    /// Получить полный путь к файлу лога
    pub fn log_file(&self) -> PathBuf {
        Path::new(self.out_dir()).join(LOG_FILE)
    }

    /// Получить полный путь к временному файлу заданий
    pub fn temp_job_file(&self) -> PathBuf {
        Path::new(self.out_dir()).join(TEMP_JOB_FILE)
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
