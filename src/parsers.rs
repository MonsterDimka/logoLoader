use crate::job_loaders::LogoJob;
use once_cell::sync::Lazy;
use regex::Regex;
use scraper::{Html, Selector};
use serde::Deserialize;
use std::error::Error;
use url::Url;

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};

#[derive(Debug)]
struct IconInfo {
    url: String,
    icon_type: String,
    sizes: Option<String>,
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum UrlType {
    JSONJob(String),
    Telegram(String),
    YandexMap(String),
    AppGallery(String),
    GooglePlay(String),
    RuStore(String),
    AppleApp(String),
    ImageUrl(String),
    VK(String),
    HH(String),
    Other(String),
    None,
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
    pub note: String,
    pub attachments: Vec<Attachment>,
}
impl DataItem {
    // Парсим json задания
    pub async fn get_job(&self) -> Result<Option<LogoJob>, Box<dyn Error>> {
        // Сначала пытаемся извлечь данные из attachments
        if let UrlType::JSONJob(url) = UrlType::from_attachments(&self.attachments) {
            return Ok(Some(LogoJob::new(self.id, url)));
        }

        // или из заметок
        match UrlType::from_note(&self.note) {
            Some(url_type) => url_type.process(self.id).await,
            None => Ok(None),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    pub url: String,
}

impl UrlType {
    // Получить Url логотипа задания для скачки
    pub fn from_note(note: &str) -> Option<UrlType> {
        note.trim()
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .filter(|s| s.starts_with("https://") || s.starts_with("http://"))
            .map(UrlType::from_str)
            .min()
    }

    pub fn from_attachments(attachments: &[Attachment]) -> UrlType {
        attachments
            .into_iter()
            .next()
            .map(|attachment| UrlType::JSONJob(attachment.url.clone()))
            .unwrap_or(UrlType::None)
    }

    pub async fn process(&self, id: u32) -> Result<Option<LogoJob>, Box<dyn Error>> {
        match self {
            UrlType::Telegram(url) => Self::process_telegram_page(id, &url).await,
            UrlType::VK(url) => Self::process_vk_page(id, &url).await,
            UrlType::RuStore(url) => Self::process_rustore_page(id, &url).await,
            UrlType::AppleApp(url) => Self::process_appstore_page(id, &url).await,
            UrlType::YandexMap(url) => {
                //<img class="img-with-alt" width="100%" height="100%" alt="Логотип" src="https://avatars.mds.yandex.net/get-altay/14031726/2a000001953765424f202da2cf59a2007866/S" style="object-fit: contain;">
                Self::process_yandex_page(id, url).await
            }
            UrlType::ImageUrl(url) => Self::process_image_page(id, url).await,
            UrlType::HH(url) => {
                // Специальная обработка для HH
                Self::process_hh_page(id, url).await
            }
            UrlType::Other(url) => Self::process_web_page(id, &url).await,
            _ => Ok(None),
        }
    }

    async fn process_telegram_page(id: u32, url: &str) -> Result<Option<LogoJob>, Box<dyn Error>> {
        println!("Не обрабатываем Телегу: {}", url);
        Ok(Some(LogoJob::new(id, url.to_string())))
        // const TELEGRAM_SELECTOR: &str = "img.tgme_page_photo_image";
        // let html = reqwest::get(url).await?.text().await?;
        // let document = Html::parse_document(&html);
        // let selector = Selector::parse(TELEGRAM_SELECTOR)?;
        // println!("Обрабатываем Телеграм: {}  {selector:?} ", url);
        //
        // document
        //     .select(&selector)
        //     .next()
        //     .and_then(|img| {
        //         println!("Найдено img {img:?}");
        //
        //         img.value().attr("src")
        //     })
        //     .map(|src| {
        //         println!("Найдено src {src}");
        //         LogoJob::new(id, src.to_string())
        //     })
        //     .map(Some)
        //     .ok_or_else(|| {
        //         Box::new(std::io::Error::new(
        //             std::io::ErrorKind::NotFound,
        //             "Изображение не найдено на странице Telegram",
        //         )) as Box<dyn Error>
        //     })
    }

    async fn process_vk_page(id: u32, url: &str) -> Result<Option<LogoJob>, Box<dyn Error>> {
        // https://smm-e.ru/services/vk/groups/download-cover/
        println!("Не обрабатываем VK: {}", url);
        Ok(Some(LogoJob::new(id, url.to_string())))
    }

    async fn process_rustore_page(id: u32, url: &str) -> Result<Option<LogoJob>, Box<dyn Error>> {
        const RUSTORE_SELECTOR: &str = r#"img[data-testid="icon"]"#;
        println!("Не обрабатываем RuStore: {}", url);

        // let html = reqwest::get(url).await?.text().await?;
        // let document = Html::parse_document(&html);
        // let selector = Selector::parse(RUSTORE_SELECTOR)?;
        //
        // for img in document.select(&selector) {
        //     println!("selector {selector:?}");
        //     if let Some(srcset) = img.value().attr("srcset") {
        //         println!("srcset {srcset:?}");
        //
        //         let first_url = srcset
        //             .split(',')
        //             .next()
        //             .map(|s| s.trim())
        //             .filter(|s| !s.is_empty())
        //             .unwrap_or(url);
        //
        //         return Ok(Some(LogoJob::new(id, first_url.to_string())));
        //     }
        // }

        Ok(Some(LogoJob::new(id, url.to_string())))
    }

    async fn process_appstore_page(id: u32, url: &str) -> Result<Option<LogoJob>, Box<dyn Error>> {
        println!("Не обрабатываем AppStore: {}", url);
        Ok(Some(LogoJob::new(id, url.to_string())))

        // const APPSTORE_SELECTOR: &str = r#"source[srcset]"#;
        // let html = reqwest::get(url).await?.text().await?;
        // let document = Html::parse_document(&html);
        // let selector = Selector::parse(APPSTORE_SELECTOR)?;
        // println!("Обрабатываем AppStore: {} {selector:?}", url);
        //
        // for source in document.select(&selector) {
        //     if let Some(srcset) = source.value().attr("srcset") {
        //         println!("Найден srcset: {}", srcset);
        //
        //         // Ищем URL с размером 400x400
        //         let urls: Vec<&str> = srcset.split(',').map(|s| s.trim()).collect();
        //
        //         for url_entry in urls {
        //             // URL в формате "https://.../400x400bb-75.webp 400w"
        //             if url_entry.contains("400x400") || url_entry.ends_with("400w") {
        //                 // Извлекаем только URL (часть до пробела)
        //                 if let Some(image_url) = url_entry.split_whitespace().next() {
        //                     println!("Найден URL 400x400: {}", image_url);
        //                     return Ok(Some(LogoJob::new(id, image_url.to_string())));
        //                 }
        //             }
        //         }
        //
        //         // Если не нашли 400x400, берем первый URL из srcset
        //         if let Some(first_url) = srcset
        //             .split(',')
        //             .next()
        //             .and_then(|s| s.split_whitespace().next())
        //         {
        //             println!("Используем первый URL из srcset: {}", first_url);
        //             return Ok(Some(LogoJob::new(id, first_url.to_string())));
        //         }
        //     }
        // }
        //
        // // Если не нашли в source, ищем в img
        // let img_selector = Selector::parse("img[src]")?;
        // for img in document.select(&img_selector) {
        //     if let Some(src) = img.value().attr("src") {
        //         if !src.is_empty() && !src.contains("1x1.gif") {
        //             println!("Найден fallback URL: {}", src);
        //             return Ok(Some(LogoJob::new(id, src.to_string())));
        //         }
        //     }
        // }
        //
        // Ok(None)
    }

    async fn process_web_page(id: u32, url: &str) -> Result<Option<LogoJob>, Box<dyn Error>> {
        println!("Не обрабатываем веб страницы: {}", url);
        Ok(Some(LogoJob::new(id, url.to_string())))

        // let response = reqwest::get(url).await?;
        // let html_content = response.text().await?;
        // let document = Html::parse_document(&html_content);
        //
        // let mut icons = Vec::new();
        // let base_url = Url::parse(url)?;
        //
        // // Поиск различных типов иконок
        // let icon_selectors = [
        //     ("icon", "link[rel='icon']"),
        //     ("shortcut icon", "link[rel='shortcut icon']"),
        //     ("apple-touch-icon", "link[rel='apple-touch-icon']"),
        //     ("mask-icon", "link[rel='mask-icon']"),
        //     ("fluid-icon", "link[rel='fluid-icon']"),
        // ];
        //
        // for (icon_type, selector_str) in icon_selectors.iter() {
        //     if let Ok(selector) = Selector::parse(selector_str) {
        //         for element in document.select(&selector) {
        //             if let Some(href) = element.value().attr("href") {
        //                 if let Ok(absolute_url) = base_url.join(href) {
        //                     let sizes = element.value().attr("sizes").map(String::from);
        //                     icons.push(IconInfo {
        //                         url: absolute_url.to_string(),
        //                         icon_type: icon_type.to_string(),
        //                         sizes,
        //                     });
        //                 }
        //             }
        //         }
        //     }
        // }
    }

    async fn process_image_page(id: u32, url: &str) -> Result<Option<LogoJob>, Box<dyn Error>> {
        println!("Обработка ссылки на картинку {url}");
        Ok(Some(LogoJob::new(id, url.to_string())))

        // println!("Обрабатываем URL на сайте: {}", url);
        // let icons = Self::process_web_page(id, &url).await?;
        // if icons.is_empty() {
        //     println!("Иконки не найдены");
        // } else {
        //     for icon in icons {
        //         println!(
        //             "Найдены иконки Тип: {}, Размеры: {:?}, URL: {}",
        //             icon.icon_type, icon.sizes, icon.url
        //         );
        //     }
        // },
        // Ok(Some(LogoJob::new(id, url.to_string())))
    }

    async fn process_yandex_page(id: u32, url: &str) -> Result<Option<LogoJob>, Box<dyn Error>> {
        println!("Не обработываем yandex страницы {url}");
        return Ok(Some(LogoJob::new(id, url.to_string())));
        let html = reqwest::get(url).await?.text().await?;
        let document = Html::parse_document(&html);

        // Создаем хеш из URL для имени файла
        let mut hasher = DefaultHasher::new();
        url.hash(&mut hasher);
        let hash = hasher.finish();
        let filename = format!("page_{}.html", hash);

        fs::write(&filename, &html)?;
        println!("HTML сохранен в файл: {}", filename);

        // Если не нашли по точному селектору, пробуем более общий
        let selector = Selector::parse(r#"img[alt="Логотип"]"#)?;
        for img in document.select(&selector) {
            println!("лого яндекc найдено {:?}", img);
            if let Some(src) = img.value().attr("src") {
                if !src.is_empty() {
                    println!("Найдено через fallback: {}", src);
                    return Ok(Some(LogoJob::new(id, src.to_string())));
                }
            }
        }

        println!("Логотип не найден, используем URL страницы");
        Ok(None)
    }
    async fn process_hh_page(id: u32, url: &str) -> Result<Option<LogoJob>, Box<dyn Error>> {
        println!("Не обрабатываем  hh страницы {url}");
        Ok(Some(LogoJob::new(id, url.to_string())))
    }

    fn from_str(url: &str) -> Self {
        // Регулярное выражение создается один раз
        static IMAGE_REGEX: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"(?i)\.(jpg|jpeg|png|gif|webp|svg|ico)(\?|$|/)").unwrap());

        // Константные паттерны для поиска
        const PATTERNS: &[(&str, fn(String) -> UrlType)] = &[
            ("https://vk.com/", UrlType::VK),
            ("https://t.me/", UrlType::Telegram),
            ("https://www.rustore.ru/", UrlType::RuStore),
            ("https://apps.apple.com/", UrlType::AppleApp),
            ("https://appgallery.huawei.com/", UrlType::AppGallery),
            ("https://play.google.com/store/apps/", UrlType::GooglePlay),
            ("https://yandex.ru/maps/", UrlType::YandexMap),
            ("https://*.hh.ru/", UrlType::HH),
        ];

        // Если есть картинка
        if IMAGE_REGEX.is_match(&url.to_lowercase()) {
            return UrlType::ImageUrl(url.to_string());
        }

        // Типизируем по url
        for (pattern, constructor) in PATTERNS {
            if url.starts_with(pattern) {
                return constructor(url.to_string());
            }
        }

        UrlType::Other(url.to_string())
    }
}

fn select_best_icon(icons: &[IconInfo]) -> Option<&IconInfo> {
    // Приоритет по размеру (ищем наибольший)
    icons
        .iter()
        .max_by_key(|icon| icon.sizes.as_ref().and_then(|s| parse_size(s)).unwrap_or(0))
        .or_else(|| icons.first())
}

fn parse_size(sizes: &str) -> Option<u32> {
    // Парсит строку размеров типа "32x32" или "32x32 64x64"
    sizes
        .split_whitespace()
        .filter_map(|size| {
            let parts: Vec<&str> = size.split('x').collect();
            if parts.len() == 2 {
                parts[0].parse::<u32>().ok()
            } else {
                None
            }
        })
        .max()
}
