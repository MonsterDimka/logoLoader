use reqwest::{Client, Error as ReqwestError, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResult {
    otp_required: bool,
    otp_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AuthenticationError {
    status_code: u16,
    pub(crate) message: String,
}

impl AuthenticationError {
    fn new(status_code: u16, message: String) -> Self {
        Self {
            status_code,
            message,
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

pub struct AuthenticationService {
    // Конфигурация
    login_url: String,
    otp_login_url: String,

    // Состояние
    otp: Arc<RwLock<Option<LoginResult>>>,

    // HTTP клиент
    pub http_client: Client,

    // Карта ошибок
    login_error_map: HashMap<u16, String>,
}

impl AuthenticationService {
    pub fn new(base_url: &str) -> Self {
        let mut error_map = HashMap::new();
        error_map.insert(403, "Неверный логин или пароль".to_string());
        error_map.insert(504, "Сервер недоступен".to_string());
        error_map.insert(666, "Неверный код".to_string());

        Self {
            // Создаём клиент с поддержкой cookies
            login_url: format!("{}/service/login", base_url),
            otp_login_url: format!("{}/service/otp", base_url),
            otp: Arc::new(RwLock::new(None)),
            http_client: Client::builder()
                // .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
                .cookie_store(true)
                .build()
                .unwrap(),
            login_error_map: error_map,
        }
    }

    pub async fn login(&self, username: &str, password: &str) -> Result<(), AuthenticationError> {
        let url = format!(
            "{}?username={}&password={}",
            self.login_url,
            urlencoding::encode(username),
            urlencoding::encode(password)
        );

        println!("Sending request to: {}", url);

        match self
            .http_client
            .post(&url)
            .header(reqwest::header::USER_AGENT, "MyTauriApp/1.0")
            // .header(reqwest::header::USER_AGENT, "Mozilla/5.0 (Tauri App)")
            // .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&serde_json::json!({}))
            .send()
            .await
        {
            Ok(response) => {
                let status = response.status();
                if status.is_success() {
                    match response.json::<LoginResult>().await {
                        Ok(result) => {
                            println!("Login result {:?}", result);
                            Ok(())
                        }
                        Err(e) => Err(AuthenticationError::new(500, e.to_string())),
                    }
                } else {
                    Err(AuthenticationError::new(
                        status.as_u16(),
                        status.to_string(),
                    ))
                }
            }
            Err(e) => Err(AuthenticationError::new(504, e.to_string())),
        }
    }

    pub async fn login_otp(&self, code: &str) -> Result<(), AuthenticationError> {
        let url = format!("{}?code={}", self.otp_login_url, urlencoding::encode(code));
        println!("Otp POST {url}");

        match self.http_client.post(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    Ok(())
                } else {
                    Err(AuthenticationError::new(
                        response.status().as_u16(),
                        response.status().to_string(),
                    ))
                }
            }
            Err(e) => Err(AuthenticationError::new(504, e.to_string())),
        }
    }
}
