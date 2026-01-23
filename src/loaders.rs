use crate::LogoJob;
use crate::parsers::UrlType;
use futures::future::join_all;
use log::info;
use reqwest;
use scraper::{Html, Selector};
use serde::Deserialize;
use std::error::Error;
use std::path::Path;
use std::{fs, io};

const TEMP_JOB_FILE: &str = "job.json";

// Загрузка задачи по созданию логотипов
pub fn simple_load_job(json_file_path: &str) -> Result<Vec<LogoJob>, Box<dyn Error + Send + Sync>> {
    let json_content = fs::read_to_string(json_file_path)?;
    let logos: Vec<LogoJob> = serde_json::from_str(&json_content)?;
    info!("Загружено заданий {}", logos.len());
    Ok(logos)
}

pub fn generate_job(dir_path: &str) -> Result<Vec<LogoJob>, Box<dyn Error + Send + Sync>> {
    const EMPTY_URL: &str = "None url";
    let path = Path::new(dir_path);

    if !path.exists() || !path.is_dir() {
        return Err(format!("Директории '{}' не существует", dir_path).into());
    }

    let image_extensions = ["jpg", "jpeg", "png", "gif", "webp"];

    let mut jobs: Vec<LogoJob> = fs::read_dir(path)?
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

    jobs.sort_by(|a, b| a.id.cmp(&b.id));
    println!("Создано {:?} задач", jobs);

    Ok(jobs)
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    pub data: Data,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Data {
    pub data: Vec<DataItem>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DataItem {
    pub id: u32,
    pub created: f64,
    pub updated: f64,
    pub username: String,
    pub merchant_id: i64,
    pub note: String,
    pub status: String,
    pub priority: String,
    pub logo: Option<serde_json::Value>,
    pub logo_attachment: Option<serde_json::Value>,
    pub attachments: Vec<Attachment>,
    pub merchant: Option<Merchant>,
    #[serde(rename = "$$hashKey")]
    pub hash_key: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    pub id: i64,
    pub url: String,
    #[serde(rename = "$$hashKey")]
    pub hash_key: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Merchant {
    pub merchant_id: i64,
    pub date_created: Option<i64>,
    pub last_updated: Option<i64>,
    pub name: String,
    pub url: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub logo_url: Option<String>,
    pub logo: Option<serde_json::Value>,
    pub logo_id: Option<i64>,
    pub rating: Option<f64>,
    pub avg_bill: Option<f64>,
    pub default_image_url: Option<String>,
    pub has_location: bool,
    pub index: i64,
    pub operations_count: i64,
    pub comment: Option<String>,
    pub skip_doubles: bool,
    pub inn_count: i64,
}

pub async fn load_json_job(
    json_file_path: &str,
) -> Result<Vec<LogoJob>, Box<dyn Error + Send + Sync>> {
    println!("Скачка файла {}", json_file_path);
    // Чтение файла с обработкой возможных ошибок
    let json_content = match fs::read_to_string(json_file_path) {
        Ok(content) => {
            let file_size = content.len();
            println!("Файл прочитан успешно, размер: {} байт", file_size);
            content
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Err(format!("Файл не найден: {}", json_file_path).into());
        }
        Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
            return Err(format!("Нет прав на чтение файла: {}", json_file_path).into());
        }
        Err(e) => {
            return Err(format!("Ошибка чтения файла '{}': {}", json_file_path, e).into());
        }
    };

    let logos: Root = match serde_json::from_str::<Root>(&json_content) {
        Ok(data) => data,
        Err(e) => {
            // Можно добавить логирование сырого JSON для отладки
            if json_content.len() < 1000 {
                eprintln!("Содержимое файла: {}", json_content);
            }
            return Err(format!("Ошибка парсинга JSON (файл: {}): {}", json_file_path, e).into());
        }
    };

    let futures: Vec<_> = logos.data.data.iter().map(|x| x.get_job()).collect();

    let results = join_all(futures).await;
    let logos: Vec<LogoJob> = results
        .into_iter()
        .filter_map(Result::ok)
        .flatten()
        .collect();

    // println!("loading {:?}", logos);

    // Сохранить задачу навсякий случай
    let json_content = serde_json::to_string_pretty(&logos)?;
    fs::write(TEMP_JOB_FILE, json_content)?;

    Ok(logos)
}

impl DataItem {
    // Парсим json задания
    async fn get_job(&self) -> Result<Option<LogoJob>, Box<dyn Error>> {
        // Попробовать получить URL из вложений
        if let UrlType::JSONJob(url) = UrlType::from_attachments(self.attachments.clone()) {
            return Ok(Some(LogoJob::new(self.id, url)));
        }

        // Обработка заметки
        let note_url = UrlType::from_note(&self.note);
        match note_url {
            Some(t) => t.process(self.id).await,
            _ => Ok(None),
        }
    }
}
