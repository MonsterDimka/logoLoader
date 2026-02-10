use crate::config::Config;
use crate::job_loaders::{Jobs, LogoJob};
use futures::stream::{self, StreamExt};
use image::ImageFormat;
use log::{error, info};
use std::error::Error;
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

const DOWNLOAD_CONCURRENCY: usize = 16;

// Скачать все изображения с сервера
pub async fn download_images(
    job: &Jobs,
    config: &Config,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let client = reqwest::Client::new();
    let download_folder = config.download_folder();
    let rework_folder = config.rework_svg_folder();

    let results: Vec<Result<(), Box<dyn Error + Send + Sync>>> =
        stream::iter(job.logos.iter().cloned().enumerate())
            .map(|(idx, logo)| {
                let client = client.clone();
                let download_folder = download_folder.clone();
                let rework_folder = rework_folder.clone();
                async move {
                    download_single_logo(&client, idx, &logo, &download_folder, &rework_folder)
                        .await
                }
            })
            .buffer_unordered(DOWNLOAD_CONCURRENCY)
            .collect()
            .await;

    // Прерываемся на первой ошибке (можно поменять на накопление/summary при желании)
    for result in results {
        result?;
    }
    Ok(())
}

async fn download_single_logo(
    client: &reqwest::Client,
    idx: usize,
    logo: &LogoJob,
    download_folder: &Path,
    rework_folder: &Path,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let response = client.get(&logo.url).send().await?;

    if response.status().is_success() {
        let is_svg = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|ct| ct.starts_with("image/svg+xml"))
            || logo.url.to_lowercase().contains(".svg");

        // Контракт хранения:
        // - растровые: download_folder/<id> (без расширения)
        // - svg: rework_folder/<id>.svg
        let out_path: PathBuf = if is_svg {
            rework_folder.join(format!("{}.svg", logo.id))
        } else {
            download_folder.join(format!("{}.png", logo.id)) // Теперь с расширением .png
        };

        let bytes = response.bytes().await?;

        if is_svg {
            // Для SVG просто сохраняем как есть
            let mut file = File::create(&out_path).await?;
            file.write_all(&bytes).await?;
        } else {
            // Для растровых изображений конвертируем в PNG
            let img = image::load_from_memory(&bytes)
                .map_err(|e| format!("Ошибка загрузки изображения: {}", e))?;
            // Сохраняем в PNG
            img.save_with_format(&out_path, ImageFormat::Png)
                .map_err(|e| format!("Ошибка сохранения PNG: {}", e))?;
        }

        info!(
            "{idx} Файл '{}' -> {} успешно скачан",
            logo.url,
            out_path.display()
        );
    } else {
        let status = response.status();
        let msg = format!("Ошибка загрузки '{}'. Код статуса: {}", logo.url, status);
        error!("{msg}");
        return Err(msg.into());
    }

    Ok(())
}
