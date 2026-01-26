use crate::config::Config;
use crate::job_loaders::Jobs;
use log::info;
use std::error::Error;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

// Скачать все изображения с сервера
pub async fn download_images(job: Jobs, config: &Config) {
    let client = reqwest::Client::new();
    let mut tasks = Vec::new();
    let mut counter = 0;
    let download_folder = config.download_folder();

    for logo in job.logos {
        let client = client.clone();
        let download_folder = download_folder.clone();
        tasks.push(tokio::spawn(async move {
            // Делаем HEAD запрос сначала, чтобы получить Content-Type
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
                            _ => "none", // fallback
                        }
                    } else {
                        "none"
                    }
                }
                Err(_) => "none",
            };

            let filename = download_folder.join(format!("{}.{}", logo.id, extension));
            let _ = get_image_by_job(&logo.url, &filename).await;
            info!(
                "{} Файл '{}' -> {} успешно скачан",
                counter,
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
