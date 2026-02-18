use crate::otp::AuthenticationService;
use crate::parsers::{Data, Root};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

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
    pub fn empty() -> Self {
        Self { logos: Vec::new() }
    }

    /// Загрузка задачи по созданию логотипов
    pub fn load_database_json_job(
        json_file_path: &str,
    ) -> Result<Jobs, Box<dyn std::error::Error + Send + Sync>> {
        let json_content = fs::read_to_string(json_file_path)?;
        let logos: Vec<LogoJob> = serde_json::from_str(&json_content)?;
        println!("Загружено заданий: {}", logos.len());
        Ok(Jobs { logos })
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
                p.file_stem()
                    .and_then(|stem| stem.to_str())
                    .and_then(|stem| stem.parse::<u32>().ok())
                    .map(|id| LogoJob::new(id, EMPTY_URL.to_string()))
            })
            .collect();

        println!("Создано заданий: {}", logos.len());
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

        println!("Обнаружено заданий {}", logos.len());

        let jobs = Jobs { logos };
        jobs
    }

    pub async fn load_from_server() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let auth_service = AuthenticationService::new("https://app.advisa.ru/master");

        // // Проверка авторизации
        // auth_service.check_login().await;
        println!("Авторизация на ADVISA");
        let login = std::env::var("login").expect("Environment variable 'login' not set");
        let password = std::env::var("password").expect("Environment variable 'password' not set");

        // Логин
        match auth_service.login(&login, &password).await {
            Ok(_) => {
                println!("Успешный логин. Введите одноразовый код:");
                let mut code = String::new();
                std::io::stdin()
                    .read_line(&mut code)
                    .expect("Failed to read line");
                let code = code.trim();

                // OTP логин
                if auth_service.is_otp_required().unwrap_or(false) {
                    match auth_service.login_otp(code).await {
                        Ok(_) => println!("OTP login successful"),
                        Err(e) => println!("OTP login failed: {}", e.message()),
                    }
                }
            }
            Err(e) => println!("Login failed: {}", e.message),
        }

        #[derive(Debug, Serialize, Deserialize)]
        struct LogoRequest {
            count: i32,
            from: i32,
            order_by: String,
            direction: String,
            priority: String,
            statuses: Vec<String>,
            filter: String,
        }

        // URL для запроса
        let url = "https://app.advisa.ru/master/service/logoRequest/list";

        // Формируем тело запроса
        let request_body = LogoRequest {
            count: 1000,
            from: 0,
            order_by: "CREATED".to_string(),
            direction: "DESC".to_string(),
            priority: "HIGH".to_string(),
            statuses: vec!["OPEN".to_string()],
            filter: "".to_string(),
        };

        // Отправляем POST запрос
        let response = auth_service
            .http_client
            .post(url)
            .json(&request_body)
            .send()
            .await?;

        // Проверяем статус ответа
        if response.status().is_success() {
            // Читаем ответ как текст (можно также десериализовать в структуру)
            let response_text = response.text().await?;
            // println!("Ответ от сервера: {}", response_text);

            let data: Data = serde_json::from_str::<Data>(&response_text)?;
            // println!("data: {data:?}");
            let jobs = Self::json_to_jobs(&data);
            // Сохранить задачу на всякий случай

            // let api_response: Root = response.json().await?;
            // println!("Ответ от сервера: {:?}", api_response);
            //
            // let jobs = Self::json_to_jobs(&api_response);
            // println!("{:?}", api_response);
            // Ok(jobs)510099
            // Ok(Self::empty())
            Ok(jobs)
        } else {
            println!("Ошибка запроса: {}", response.status());
            let error_text = response.text().await?;
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
