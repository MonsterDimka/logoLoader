use crate::otp::AuthenticationService;
use crate::parsers::{Data, Root};
use chrono::{DateTime, Datelike, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct LogoJob {
    pub url: String,
    pub id: u32,
    pub note: String,
}

impl LogoJob {
    pub fn new(id: u32, url: String, note: String) -> Self {
        Self { id, url, note }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct LogoRequest {
    count: i32,
    from: i32,
    order_by: String,
    direction: String,
    priority: Option<String>,
    statuses: Vec<String>,
    filter: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Jobs {
    pub logos: Vec<LogoJob>,
}

impl Jobs {
    pub fn empty() -> Self {
        Self { logos: Vec::new() }
    }

    /// Загрузка задачи по созданию логотипов
    pub fn load_database_json_job(
        json_file_path: &str,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let json_content = fs::read_to_string(json_file_path)?;
        let logos: Vec<LogoJob> = serde_json::from_str(&json_content)?;
        println!("Загружено заданий: {}", logos.len());
        Ok(Jobs { logos })
    }

    //поиск терминала по id
    pub fn is_terminal(&self, id: u32) -> bool {
        let TERMINAl = "для терминалов";
        //Логотип для терминалов, не должен быть на белом фоне
        self.logos
            .iter()
            .find(|x| x.id == id)
            .is_some_and(|x| x.note.contains(TERMINAl))
    }

    /// Создание задачи по обработке логотипов на основе изображений из директории
    pub fn generate_job_from_dir_images(
        dir_path: &str,
    ) -> Result<Jobs, Box<dyn std::error::Error + Send + Sync>> {
        const EMPTY_URL: &str = "None url";
        let path = Path::new(dir_path);

        if !path.exists() || !path.is_dir() {
            return Err(
                format!("Директории {dir_path} для генерации задания не существует").into(),
            );
        }
        let image_extensions = ["jpg", "jpeg", "png", "gif", "webp"];

        let logos: Vec<LogoJob> = fs::read_dir(path)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().is_file())
            .filter(|entry| {
                // Поддерживаем оба варианта:
                // - файлы с расширением (png/jpg/...)
                // - файлы без расширения (контракт скачивания: download_folder/<id>)
                match entry.path().extension().and_then(|ext| ext.to_str()) {
                    Some(ext) => image_extensions.contains(&ext.to_lowercase().as_str()),
                    None => true,
                }
            })
            .filter_map(|entry| {
                let p = entry.path();
                // Если расширения нет, file_stem() вернёт имя файла целиком.
                println!(
                    "Файл {:#?}",
                    p.file_stem().unwrap().to_str().unwrap().parse::<u32>()
                );

                p.file_stem()
                    .and_then(|stem| stem.to_str())
                    .and_then(|stem| stem.parse::<u32>().ok())
                    .map(|id| LogoJob::new(id, EMPTY_URL.to_string(), String::new()))
            })
            .collect();

        println!("Создано заданий: {} {:#?}", logos.len(), logos);
        Ok(Jobs { logos })
    }

    /// Загрузка задачи по созданию логотипов
    pub fn load_json_job(
        json_text: &str,
        json_file_path: &str,
        temp_job_path: &PathBuf,
        backup: bool,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        println!("Скачка файла {}", json_file_path);
        let json_content = if json_text.is_empty() {
            fs::read_to_string(json_file_path)?
        } else {
            json_text.to_string()
        };

        let root: Root = serde_json::from_str::<Root>(&json_content)?;
        let jobs = Self::json_to_jobs(&root.data);
        // Сохранить задачу на всякий случай

        if backup {
            jobs.jobs_backup(temp_job_path)?;
        }
        Ok(jobs)
    }

    fn json_to_jobs(root: &Data) -> Jobs {
        let logos: Vec<LogoJob> = root
            .data
            .iter()
            .filter_map(|x| x.get_job().ok())
            .flatten()
            .collect();

        // println!("Обнаружено заданий {:?} {}", logos, logos.len());

        let jobs = Jobs { logos };
        // println!("Задания: \n {:?}", jobs);

        jobs
    }

    pub async fn load_from_server(
        login: &str,
        password: &str,
        otp_code: Option<String>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let auth_url = "https://app.advisa.ru/master";
        let auth_service = AuthenticationService::new(auth_url);

        println!("Авторизация на ADVISA");

        // Логин
        match auth_service.login(&login, &password).await {
            Ok(_) => {
                println!("Успешный логин. Пожалуйста введите одноразовый код:");

                let code = otp_code.unwrap_or_else(|| {
                    let mut otp_code = String::new();
                    std::io::stdin()
                        .read_line(&mut otp_code)
                        .expect("Ошибка ввода одноразового пароля");
                    otp_code.trim().to_string()
                });

                // OTP логин
                match auth_service.login_otp(code.as_str()).await {
                    Ok(_) => println!("OTP успешный"),
                    Err(e) => println!("Ошибка OTP пароля: {}", e.message()),
                }
            }
            Err(e) => println!("Ошибка логина: {}", e.message),
        }

        // URL для запроса
        let url = "https://app.advisa.ru/master/service/logoRequest/list";

        // Формируем тело запроса на новые логотипы
        let new_logo_request = LogoRequest {
            count: 1500,
            from: 0,
            order_by: "CREATED".to_string(),
            direction: "DESC".to_string(),
            // priority: None,
            priority: Some("HIGH".to_string()), // только логотипы с высоким приоритетом логотипы
            statuses: vec!["OPEN".to_string()],
            filter: "".to_string(),
        };

        // Формируем тело запроса на новые логотипы
        let ready_logo_request = LogoRequest {
            count: 500,
            from: 0,
            order_by: "UPDATED".to_string(),
            direction: "DESC".to_string(),
            priority: None,
            statuses: vec!["DONE".to_string()],
            filter: "".to_string(),
        };

        // Отправляем POST запрос
        let response_new_logo = auth_service
            .http_client
            .post(url)
            .json(&new_logo_request)
            .send()
            .await?;

        // Проверяем статус ответа
        if response_new_logo.status().is_success() {
            // Читаем ответ как текст (можно также десериализовать в структуру)
            let response_new_logo_text = response_new_logo.text().await?;
            let new_logo_data: Data = serde_json::from_str::<Data>(&response_new_logo_text)?;
            let jobs = Self::json_to_jobs(&new_logo_data);

            // кусочек подсчета сделанных логотипов
            // let response_ready_logo = auth_service
            //     .http_client
            //     .post(url)
            //     .json(&ready_logo_request)
            //     .send()
            //     .await?;
            //
            // let response_ready_logo_text = response_ready_logo.text().await?;
            // let ready_logo_data: Data = serde_json::from_str::<Data>(&response_ready_logo_text)?;
            // Self::json_count(&ready_logo_data);

            Ok(jobs)
        } else {
            println!("Ошибка запроса: {}", response_new_logo.status());
            let error_text = response_new_logo.text().await?;
            println!("Детали ошибки: {}", error_text);
            Ok(Self::empty())
        }
    }

    /// Сохраняет список заданий в JSON по указанному пути (резервная копия).
    fn jobs_backup(&self, path: &Path) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let json = serde_json::to_string_pretty(&self.logos)?;
        fs::write(path, json)?;
        Ok(())
    }
}
