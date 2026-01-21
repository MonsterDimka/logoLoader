use crate::LogoJob;
use log::info;
use serde::Deserialize;
use std::error::Error;
use std::{fs, io};

// Загрузка задачи по созданию логотипов
fn load_job(json_file_path: &str) -> Result<Vec<LogoJob>, Box<dyn Error + Send + Sync>> {
    let json_content = fs::read_to_string(json_file_path)?;
    let logos: Vec<LogoJob> = serde_json::from_str(&json_content)?;
    info!("Загружено заданий {}", logos.len());
    Ok(logos)
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
    pub merchant: Merchant,
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
    pub date_created: i64,
    pub last_updated: i64,
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

pub fn load_json_job(json_file_path: &str) -> Result<Vec<LogoJob>, Box<dyn Error + Send + Sync>> {
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

    let logos = logos
        .data
        .data
        .iter()
        .map(|x| LogoJob {
            id: x.id,
            url: x.attachments.iter().next().unwrap().url.clone(),
        })
        .collect();

    Ok(logos)
}
