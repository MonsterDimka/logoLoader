use reqwest::{Client, Error as ReqwestError, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    name: String,
    permissions: Vec<Permission>,
    bank_id: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Permission {
    // Добавьте ваши разрешения здесь
    Admin,
    User,
    Manager,
    // ... другие разрешения
}

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
    user_info_url: String,
    login_url: String,
    otp_login_url: String,

    // Состояние
    user: Arc<RwLock<Option<User>>>,
    last_error: Arc<RwLock<Option<AuthenticationError>>>,
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
            user_info_url: format!("{}/service/user/info", base_url),
            login_url: format!("{}/service/login", base_url),
            otp_login_url: format!("{}/service/otp", base_url),
            user: Arc::new(RwLock::new(None)),
            last_error: Arc::new(RwLock::new(None)),
            otp: Arc::new(RwLock::new(None)),
            http_client: Client::builder().cookie_store(true).build().unwrap(),
            login_error_map: error_map,
        }
    }

    // Геттеры для computed значений (аналог signal + computed)
    pub fn user_name(&self) -> String {
        self.user
            .read()
            .unwrap()
            .as_ref()
            .map(|u| u.name.clone())
            .unwrap_or_default()
    }
    pub fn is_logged(&self) -> bool {
        self.user.read().unwrap().is_some()
    }

    pub fn is_otp_required(&self) -> Option<bool> {
        self.otp.read().unwrap().as_ref().map(|o| o.otp_required)
    }

    pub fn get_otp_url(&self) -> String {
        self.otp
            .read()
            .unwrap()
            .as_ref()
            .and_then(|o| o.otp_url.clone())
            .unwrap_or_default()
    }

    pub fn error_message(&self) -> String {
        self.last_error
            .read()
            .unwrap()
            .as_ref()
            .map(|e| {
                self.login_error_map
                    .get(&e.status_code)
                    .cloned()
                    .unwrap_or_else(|| "Неизвестная ошибка".to_string())
            })
            .unwrap_or_default()
    }

    pub async fn check_login(&self) {
        self.get_user_info().await;
    }

    pub async fn login(&self, username: &str, password: &str) -> Result<(), AuthenticationError> {
        let url = format!(
            "{}?username={}&password={}",
            self.login_url,
            urlencoding::encode(username),
            urlencoding::encode(password)
        );

        match self.http_client.post(&url).json("{}").send().await {
            Ok(response) => {
                let status = response.status();
                if status.is_success() {
                    match response.json::<LoginResult>().await {
                        Ok(result) => {
                            println!("Login result {:?}", result);
                            self.login_success(result);
                            Ok(())
                        }
                        Err(e) => {
                            self.login_failed(&e);
                            Err(AuthenticationError::new(500, e.to_string()))
                        }
                    }
                } else {
                    let error = AuthenticationError::new(status.as_u16(), status.to_string());
                    self.login_failed(&error);
                    Err(error)
                }
            }
            Err(e) => {
                let error = AuthenticationError::new(504, e.to_string());
                self.login_failed(&error);
                Err(error)
            }
        }
    }

    pub async fn login_otp(&self, code: &str) -> Result<(), AuthenticationError> {
        let url = format!("{}?code={}", self.otp_login_url, urlencoding::encode(code));
        println!("Otp POST {url}");

        match self.http_client.post(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    self.otp_login_success().await;
                    Ok(())
                } else {
                    let error = AuthenticationError::new(
                        response.status().as_u16(),
                        response.status().to_string(),
                    );
                    self.otp_login_failed(&error);
                    Err(error)
                }
            }
            Err(e) => {
                let error = AuthenticationError::new(504, e.to_string());
                self.otp_login_failed(&error);
                Err(error)
            }
        }
    }

    fn login_success(&self, otp: LoginResult) {
        *self.last_error.write().unwrap() = None;

        if !otp.otp_required {
            println!("Успешный логин");
            // Здесь нужно вызвать tokio::spawn для асинхронного вызова
            let service = self.clone();
            tokio::spawn(async move {
                service.get_user_info().await;
            });
        } else {
            println!("Запрос дополнительного Otp пароля");
            *self.otp.write().unwrap() = Some(otp);
        }
    }

    pub fn otp_login_cancel(&self) {
        *self.last_error.write().unwrap() = None;
        *self.user.write().unwrap() = None;
        *self.otp.write().unwrap() = None;
    }

    async fn otp_login_success(&self) {
        *self.last_error.write().unwrap() = None;
        *self.otp.write().unwrap() = None;
        println!("Успешный Otp логин");
        self.get_user_info().await;
    }

    fn login_failed<T: std::fmt::Debug + 'static>(&self, error: &T) {
        *self.user.write().unwrap() = None;
        // Преобразуем ошибку в AuthenticationError если это возможно
        if let Some(e) = (error as &dyn std::any::Any).downcast_ref::<AuthenticationError>() {
            *self.last_error.write().unwrap() = Some(e.clone());
        } else if let Some(e) = (error as &dyn std::any::Any).downcast_ref::<ReqwestError>() {
            *self.last_error.write().unwrap() = Some(AuthenticationError::new(500, e.to_string()));
        }
    }

    fn otp_login_failed(&self, error: &AuthenticationError) {
        *self.otp.write().unwrap() = None;
        *self.user.write().unwrap() = None;

        let mut otp_error = error.clone();
        otp_error.status_code = 666;
        *self.last_error.write().unwrap() = Some(otp_error);
    }

    pub fn logout(&self) {
        println!("Разлогин");
        *self.user.write().unwrap() = None;
        *self.last_error.write().unwrap() = None;
        // В Rust мы не можем изменить URL браузера, но можем вернуть команду
        // или использовать какой-то механизм для редиректа
    }

    async fn get_user_info(&self) {
        match self.http_client.get(&self.user_info_url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<User>().await {
                        Ok(user) => {
                            println!("Данные пользователя успешно получены: {:?}", user);
                            *self.user.write().unwrap() = Some(user);
                        }
                        Err(_) => {
                            *self.user.write().unwrap() = None;
                        }
                    }
                } else {
                    *self.user.write().unwrap() = None;
                }
            }
            Err(_) => {
                *self.user.write().unwrap() = None;
            }
        }
    }
}

// Для возможности клонирования сервиса (аналог Injectable)
impl Clone for AuthenticationService {
    fn clone(&self) -> Self {
        Self {
            user_info_url: self.user_info_url.clone(),
            login_url: self.login_url.clone(),
            otp_login_url: self.otp_login_url.clone(),
            user: Arc::clone(&self.user),
            last_error: Arc::clone(&self.last_error),
            otp: Arc::clone(&self.otp),
            http_client: self.http_client.clone(),
            login_error_map: self.login_error_map.clone(),
        }
    }
}

// Пример использования:
#[tokio::main]
async fn main() {
    let auth_service = AuthenticationService::new("http://localhost:8080");

    // Проверка авторизации
    auth_service.check_login().await;

    // Логин
    match auth_service.login("username", "password").await {
        Ok(_) => println!("Login successful"),
        Err(e) => println!("Login failed: {}", e.message()),
    }

    // Получение имени пользователя
    println!("User name: {}", auth_service.user_name());

    // OTP логин
    if auth_service.is_otp_required().unwrap_or(false) {
        match auth_service.login_otp("123456").await {
            Ok(_) => println!("OTP login successful"),
            Err(e) => println!("OTP login failed: {}", e.message()),
        }
    }

    // Выход
    auth_service.logout();
}
