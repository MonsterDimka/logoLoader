use futures::future::join_all;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::parsers::Root;

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct LogoJob {
    pub url: String,
    pub id: u32,
}

impl LogoJob {
    pub fn new(id: u32, url: String) -> Self {
        Self { id, url }
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Jobs {
    pub logos: Vec<LogoJob>,
}

impl Jobs {
    /// Загрузка задачи по созданию логотипов
    pub fn load_database_json_job(json_file_path: &str) -> Jobs {
        let json_content = fs::read_to_string(json_file_path).expect("Ошибка чтения json файла");
        let logos: Vec<LogoJob> =
            serde_json::from_str(&json_content).expect("Ошибка парсинга json");
        println!("Загружено заданий: {}", logos.len());
        Jobs { logos }
    }

    /// Создание задачи по обработке логотипов на основе изображений из директории
    pub fn generate_job_from_dir_images(dir_path: &str) -> Jobs {
        const EMPTY_URL: &str = "None url";
        let path = Path::new(dir_path);

        if !path.exists() || !path.is_dir() {
            panic!("Директории {dir_path} для генерации задания не существует");
        }
        let image_extensions = ["jpg", "jpeg", "png", "gif", "webp"];

        let logos: Vec<LogoJob> = fs::read_dir(path)
            .expect("Ошибка чтения директории job")
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().is_file())
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| image_extensions.contains(&ext.to_lowercase().as_str()))
                    .unwrap_or(false)
            })
            .filter_map(|entry| {
                entry
                    .path()
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .and_then(|stem| stem.parse::<u32>().ok())
                    .map(|id| LogoJob::new(id, EMPTY_URL.to_string()))
            })
            .collect();

        println!("Создано заданий: {}", logos.len());
        Jobs { logos }
    }

    /// Загрузка задачи по созданию логотипов
    pub fn load_json_job(json_text: &str, json_file_path: &str, temp_job_path: &PathBuf) -> Self {
        println!("Скачка файла {}", json_file_path);
        // Чтение файла с обработкой возможных ошибок
        let json_content = if json_text.is_empty() {
            fs::read_to_string(json_file_path).expect("Ошибка чтения json файла задачи")
        } else {
            json_text.to_string()
        };

        let logos: Root =
            serde_json::from_str::<Root>(&json_content).expect("Ошибка парсинга json");

        let futures: Vec<_> = logos.data.data.iter().map(|x| x.get_job()).collect();

        // let results = join_all(futures).await;
        let logos: Vec<LogoJob> = futures
            .into_iter()
            .filter_map(Result::ok)
            .flatten()
            .collect();

        println!("Загружено заданий {}", logos.len());

        let jobs = Jobs { logos };
        // Сохранить задачу на всякий случай

        jobs.jobs_backup(temp_job_path);
        jobs
    }

    /// Сохраняет список заданий в JSON по указанному пути (резервная копия).
    fn jobs_backup(&self, path: &Path) {
        let json = serde_json::to_string_pretty(&self.logos)
            .expect("Невозможно создать запасной json задания");
        fs::write(path, json).expect("Ошибка сохранения запасного json задания");
    }
}
