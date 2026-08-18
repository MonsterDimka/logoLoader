use crate::job_loaders::LogoJob;
use once_cell::sync::Lazy;
use regex::Regex;
use scraper::{Html, Selector};
use serde::Deserialize;
use std::error::Error;
use url::Url;

use chrono::{DateTime, Utc};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};

#[derive(Debug)]
struct IconInfo {
    url: String,
    icon_type: String,
    sizes: Option<String>,
    priority: u8, // Приоритет: выше = лучше
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
    pub updated: f64,
}

impl DataItem {
    // Парсим json задания
    pub fn get_job(&self) -> Result<Option<LogoJob>, Box<dyn Error>> {
        // Сначала пытаемся извлечь данные из attachments
        if let Some(url) = Self::from_attachments(&self.attachments) {
            return Ok(Some(LogoJob::new(self.id, url, self.note.clone())));
        }

        Ok(None)
        // // или из заметок
        // match UrlType::from_note(&self.note) {
        //     Some(url_type) => url_type.process(self.id).await,
        //     None => Ok(None),
        // }
    }

    // pub fn from_note(note: &str) -> Option<UrlType> {
    //     note.trim()
    //         .lines()
    //         .map(str::trim)
    //         .filter(|s| !s.is_empty())
    //         .filter(|s| s.starts_with("https://") || s.starts_with("http://"))
    //         .map(UrlType::from_str)
    //         .min()
    // }

    pub fn from_attachments(attachments: &[Attachment]) -> Option<String> {
        attachments
            .iter()
            .max_by_key(|x| x.id)
            .map(|attachment| attachment.url.clone())
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    pub url: String,
    pub id: u32,
}
