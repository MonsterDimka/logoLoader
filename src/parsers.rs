use crate::LogoJob;
use crate::loaders::Attachment;
use regex::Regex;
use scraper::{Html, Selector};
use std::error::Error;
use url::Url;

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
    RuStore(String),
    AppleApp(String),
    VK(String),
    AppGallery(String),
    GooglePlay(String),
    ImageUrl(String),
    Other(String),
    None,
}

fn is_image_url_regex(url: &str) -> bool {
    // Регулярное выражение для поиска расширений изображений в URL
    let re = Regex::new(r"\.(jpg|jpeg|png|gif|bmp|webp|svg|tiff|ico|jfif|avif|apng)([^a-zA-Z]|$)")
        .unwrap();

    let url_lower = url.to_lowercase();
    re.is_match(&url_lower)
}

impl UrlType {
    fn from_str(url: &str) -> UrlType {
        // Более строгая проверка именно начала домена
        let patterns = [
            ("https://vk.com/", UrlType::VK(url.to_string())),
            ("https://t.me/", UrlType::Telegram(url.to_string())),
            ("https://www.rustore.ru/", UrlType::RuStore(url.to_string())),
            (
                "https://apps.apple.com/",
                UrlType::AppleApp(url.to_string()),
            ),
            (
                "https://appgallery.huawei.com/",
                UrlType::AppGallery(url.to_string()),
            ),
            (
                "https://play.google.com/store/apps/",
                UrlType::GooglePlay(url.to_string()),
            ),
        ];

        for (pattern, url_type) in &patterns {
            if url.starts_with(pattern) {
                return url_type.clone();
            }
        }

        if is_image_url_regex(url) {
            return UrlType::ImageUrl(url.to_string());
        }

        UrlType::Other(url.to_string())
    }

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

    pub fn from_attachments(attachments: Vec<Attachment>) -> UrlType {
        attachments
            .first()
            .map(|attachment| UrlType::JSONJob(attachment.url.clone()))
            .unwrap_or(UrlType::None)
    }

    pub async fn process(&self, id: u32) -> Result<Option<LogoJob>, Box<dyn Error>> {
        match self {
            UrlType::Telegram(url) => Self::process_telegram_page(id, &url).await,
            UrlType::VK(url) => Self::process_vk_page(id, &url),
            UrlType::RuStore(url) => Self::process_rustore_page(id, &url).await,
            UrlType::AppleApp(url) => Self::process_appstore_page(id, &url).await,
            UrlType::ImageUrl(url) => {
                println!("Обрабатываем URL на сайте: {}", url);
                let icons = Self::process_web_page(id, &url).await?;
                if icons.is_empty() {
                    println!("Иконки не найдены");
                } else {
                    for icon in icons {
                        println!(
                            "Найдены иконки Тип: {}, Размеры: {:?}, URL: {}",
                            icon.icon_type, icon.sizes, icon.url
                        );
                    }
                }
                Ok(Some(LogoJob::new(id, url.to_string())))
            }
            _ => Ok(None),
        }
    }

    async fn process_telegram_page(id: u32, url: &str) -> Result<Option<LogoJob>, Box<dyn Error>> {
        const TELEGRAM_SELECTOR: &str = "img.tgme_page_photo_image";
        let html = reqwest::get(url).await?.text().await?;
        let document = Html::parse_document(&html);
        let selector = Selector::parse(TELEGRAM_SELECTOR)?;
        println!("Обрабатываем Телеграм: {}  {selector:?} ", url);

        document
            .select(&selector)
            .next()
            .and_then(|img| {
                println!("Найдено img {img:?}");

                img.value().attr("src")
            })
            .map(|src| {
                println!("Найдено src {src}");
                LogoJob::new(id, src.to_string())
            })
            .map(Some)
            .ok_or_else(|| {
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Изображение не найдено на странице Telegram",
                )) as Box<dyn Error>
            })
    }

    fn process_vk_page(id: u32, url: &str) -> Result<Option<LogoJob>, Box<dyn Error>> {
        //     // https://smm-e.ru/services/vk/groups/download-cover/
        println!("Обрабатываем VK: {}", url);
        Ok(Some(LogoJob::new(id, url.to_string())))
    }

    async fn process_rustore_page(id: u32, url: &str) -> Result<Option<LogoJob>, Box<dyn Error>> {
        const RUSTORE_SELECTOR: &str = r#"img[data-testid="icon"]"#;
        let html = reqwest::get(url).await?.text().await?;
        let document = Html::parse_document(&html);
        let selector = Selector::parse(RUSTORE_SELECTOR)?;
        println!("Обрабатываем RuStore: {} {selector:?}", url);

        for img in document.select(&selector) {
            println!("selector {selector:?}");
            if let Some(srcset) = img.value().attr("srcset") {
                println!("srcset {srcset:?}");

                let first_url = srcset
                    .split(',')
                    .next()
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .unwrap_or(url);

                return Ok(Some(LogoJob::new(id, first_url.to_string())));
            }
        }

        Ok(Some(LogoJob::new(id, url.to_string())))
    }

    async fn process_appstore_page(id: u32, url: &str) -> Result<Option<LogoJob>, Box<dyn Error>> {
        println!("Обрабатываем AppStore: {}", url);
        const APPSTORE_SELECTOR: &str = r#"source[srcset]"#;
        let html = reqwest::get(url).await?.text().await?;
        let document = Html::parse_document(&html);
        let selector = Selector::parse(APPSTORE_SELECTOR)?;
        println!("Обрабатываем AppStore: {} {selector:?}", url);

        for source in document.select(&selector) {
            if let Some(srcset) = source.value().attr("srcset") {
                println!("Найден srcset: {}", srcset);

                // Ищем URL с размером 400x400
                let urls: Vec<&str> = srcset.split(',').map(|s| s.trim()).collect();

                for url_entry in urls {
                    // URL в формате "https://.../400x400bb-75.webp 400w"
                    if url_entry.contains("400x400") || url_entry.ends_with("400w") {
                        // Извлекаем только URL (часть до пробела)
                        if let Some(image_url) = url_entry.split_whitespace().next() {
                            println!("Найден URL 400x400: {}", image_url);
                            return Ok(Some(LogoJob::new(id, image_url.to_string())));
                        }
                    }
                }

                // Если не нашли 400x400, берем первый URL из srcset
                if let Some(first_url) = srcset
                    .split(',')
                    .next()
                    .and_then(|s| s.split_whitespace().next())
                {
                    println!("Используем первый URL из srcset: {}", first_url);
                    return Ok(Some(LogoJob::new(id, first_url.to_string())));
                }
            }
        }

        // Если не нашли в source, ищем в img
        let img_selector = Selector::parse("img[src]")?;
        for img in document.select(&img_selector) {
            if let Some(src) = img.value().attr("src") {
                if !src.is_empty() && !src.contains("1x1.gif") {
                    println!("Найден fallback URL: {}", src);
                    return Ok(Some(LogoJob::new(id, src.to_string())));
                }
            }
        }

        Ok(None)
    }

    async fn process_web_page(id: u32, page_url: &str) -> Result<Vec<IconInfo>, Box<dyn Error>> {
        let response = reqwest::get(page_url).await?;
        let html_content = response.text().await?;
        let document = Html::parse_document(&html_content);

        let mut icons = Vec::new();
        let base_url = Url::parse(page_url)?;

        // Поиск различных типов иконок
        let icon_selectors = [
            ("icon", "link[rel='icon']"),
            ("shortcut icon", "link[rel='shortcut icon']"),
            ("apple-touch-icon", "link[rel='apple-touch-icon']"),
            ("mask-icon", "link[rel='mask-icon']"),
            ("fluid-icon", "link[rel='fluid-icon']"),
        ];

        for (icon_type, selector_str) in icon_selectors.iter() {
            if let Ok(selector) = Selector::parse(selector_str) {
                for element in document.select(&selector) {
                    if let Some(href) = element.value().attr("href") {
                        if let Ok(absolute_url) = base_url.join(href) {
                            let sizes = element.value().attr("sizes").map(String::from);
                            icons.push(IconInfo {
                                url: absolute_url.to_string(),
                                icon_type: icon_type.to_string(),
                                sizes,
                            });
                        }
                    }
                }
            }
        }

        // Поиск Open Graph изображения
        if let Ok(selector) = Selector::parse("meta[property='og:image']") {
            for element in document.select(&selector) {
                if let Some(content) = element.value().attr("content") {
                    if let Ok(absolute_url) = base_url.join(content) {
                        icons.push(IconInfo {
                            url: absolute_url.to_string(),
                            icon_type: "og:image".to_string(),
                            sizes: None,
                        });
                    }
                }
            }
        }

        // Поиск Twitter изображения
        if let Ok(selector) = Selector::parse("meta[name='twitter:image']") {
            for element in document.select(&selector) {
                if let Some(content) = element.value().attr("content") {
                    if let Ok(absolute_url) = base_url.join(content) {
                        icons.push(IconInfo {
                            url: absolute_url.to_string(),
                            icon_type: "twitter:image".to_string(),
                            sizes: None,
                        });
                    }
                }
            }
        }

        Ok(icons)
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
