use crate::config::Config;
use crate::job_loaders::{Jobs, LogoJob};
use log::info;
use std::error::Error;
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

async fn detect_image_extension(
    client: &reqwest::Client,
    logo: &LogoJob,
    download_folder: &Path,
    rework_folder: &Path,
) -> (String, PathBuf) {
    let extension = match client.head(&logo.url).send().await {
        Ok(response) => {
            if let Some(content_type) = response.headers().get("content-type") {
                match content_type.to_str().unwrap_or("") {
                    "image/jpeg" => "jpg",
                    "image/jpg" => "jpg",
                    "image/png" => "png",
                    "image/gif" => "gif",
                    "image/webp" => "webp",
                    "image/svg+xml" => "svg",
                    _ => "none",
                }
            } else {
                "none"
            }
        }
        Err(_) => "none",
    };

    let name = format!("{}.{}", logo.id, extension);
    let filename = match extension {
        "svg" | "none" => rework_folder.join(&name),
        _ => download_folder.join(&name),
    };

    (extension.to_string(), filename)
}

// Скачать все изображения с сервера
pub async fn download_images(job: &Jobs, config: &Config) {
    let client = reqwest::Client::new();
    let mut tasks = Vec::new();
    let mut counter = 0;
    let download_folder = config.download_folder();
    let rework_folder = config.rework_svg_folder();

    for logo in &job.logos {
        let client = client.clone();
        let download_folder = download_folder.clone();
        let rework_folder = rework_folder.clone();
        let logo = logo.clone();

        tasks.push(tokio::spawn(async move {
            let (extension, filename) =
                detect_image_extension(&client, &logo, &download_folder, &rework_folder).await;
            get_image_by_job(&logo.url, &filename).await;

            info!(
                "{counter} Файл '{}' -> {} {extension} успешно скачан",
                logo.url,
                filename.display()
            );
        }));
        counter += 1;
    }

    for task in tasks {
        let _ = task.await;
    }
}

async fn get_image_by_job(
    url: &str,
    out: &std::path::Path,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let client_new = reqwest::Client::new();
    let response = client_new.get(url).send().await?;

    if response.status().is_success() {
        let bytes = response.bytes().await?;
        let mut file = File::create(out).await?;
        file.write_all(&bytes).await?;
    } else {
        println!(
            "Ошибка загрузки '{}'. Код статуса: {}",
            url,
            response.status()
        );
    }

    Ok(())
}
