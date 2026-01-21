use crate::background_works::DominantColor;
use crate::{DOWNLOAD_FOLDER, LogoJob, RESULT_FOLDER, UPSCALE_FOLDER, background_works, save};
use futures::future::join_all;
use image::{DynamicImage, GenericImageView, ImageBuffer, ImageReader, Rgb, Rgba};
use indicatif::ProgressBar;
use log::{error, info};
use palette::Srgb;
use std::error::Error;
use std::path::Path;

const WHITE_COLOR: u8 = 250;
const MIN_SCORE_DOMINANT_COLOR: f32 = 0.5;
const GRAY_BACKGROUND_COLOR: Srgb<u8> = Srgb::new(238, 237, 241);

pub async fn images_works_parallel(
    logos: Vec<LogoJob>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let bar = ProgressBar::new(logos.len() as u64);

    let mut tasks = Vec::new();
    let mut task_id = 0;

    for logo in logos {
        let bar_clone = bar.clone();

        tasks.push(tokio::spawn(async move {
            let result = process_single_logo(logo, task_id).await;
            bar_clone.inc(1);
            result
        }));
        task_id += 1;
    }

    // Ждем завершения всех задач
    let results = join_all(tasks).await;

    // Проверяем результаты
    for result in results {
        match result {
            Ok(Ok(_)) => continue,
            Ok(Err(e)) => return Err(e),
            Err(e) => return Err(Box::new(e)),
        }
    }

    info!("Обработка завершена");
    bar.finish_with_message("Обработка завершена");

    Ok(())
}

async fn process_single_logo(logo: LogoJob, task: i32) -> Result<(), Box<dyn Error + Send + Sync>> {
    let id = logo.id;

    let small_image_name = format!("{}/{}.jpg", DOWNLOAD_FOLDER, id);
    let big_image_name = format!("{}/{}.png", UPSCALE_FOLDER, id);

    info!(
        "{} Таска обработки начата. Задача:{} Файлы для обработки: {} {}",
        task, id, small_image_name, big_image_name
    );

    // Загружаем изображения
    let small_rgb_image = load_image(&small_image_name)?.to_rgb8();
    // Получение доминирующего в изображении цвета (цвета фона)
    let background = DominantColor::from_rgb_image(small_rgb_image)?;

    let big_rgba_image = load_image(&big_image_name)?.to_rgba8();
    let mut final_image = big_rgba_image;

    // Удаление фона
    if background.score > MIN_SCORE_DOMINANT_COLOR {
        background.remove_image_background(&mut final_image);
        final_image = background_works::trim_transparent_border(&mut final_image);
    }

    // Выбор цвета фона серый для белого фона и доминантный для остальных
    let background: DominantColor = if background.average > WHITE_COLOR {
        DominantColor {
            color: GRAY_BACKGROUND_COLOR,
            ..background
        }
    } else {
        background
    };

    // Формирование имени SVG файла
    let new_image_name = format!("{}/{}.svg", RESULT_FOLDER, id);
    let percent = (background.score * 100.0) as u16;

    // Логирование
    info!(
        "{} Доминирующий цвет: RGB({} {} {})  Всего: {}%",
        small_image_name,
        background.color.red,
        background.color.green,
        background.color.blue,
        percent
    );

    // Создание SVG
    let _ = save::save_ready_logo(final_image, id, background, &new_image_name, true);

    info!(
        "{} Таска закончена. Задача:{} Файлы для обработки: {} {} Сохранение {}",
        task, id, small_image_name, big_image_name, new_image_name
    );
    Ok(())
}

fn load_image(image_name: &str) -> Result<DynamicImage, Box<dyn Error + Send + Sync>> {
    // Проверка существования файла
    if !Path::new(image_name).exists() {
        error!("Файл не найден: {}", image_name);
    }

    let image = ImageReader::open(image_name)?
        .with_guessed_format()?
        .decode()?;

    // Проверяем, есть ли альфа-канал и конвертируем в RGB если нужно
    let image = match image {
        DynamicImage::ImageRgba8(rgba_image) => {
            // Конвертируем RGBA в RGB с белым фоном
            let (width, height) = rgba_image.dimensions();
            let mut rgb_image = ImageBuffer::new(width, height);

            for (x, y, pixel) in rgba_image.enumerate_pixels() {
                let Rgba([r, g, b, a]) = pixel;
                if *a == 255 {
                    // Полностью непрозрачный - просто копируем RGB
                    rgb_image.put_pixel(x, y, Rgb([*r, *g, *b]));
                } else {
                    rgb_image.put_pixel(x, y, Rgb([255, 255, 255]));
                }
            }
            DynamicImage::ImageRgb8(rgb_image)
        }

        // Для остальных типов изображений (RGB, Gray, etc.) оставляем как есть
        _ => image,
    };

    let (width, height) = image.dimensions();

    // Логирование
    info!(
        "Загружена картинка: {} Размер картинки {}x{} формат",
        image_name, width, height
    );
    Ok(image)
}
