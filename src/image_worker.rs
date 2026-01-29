use crate::background_works::{DominantColor, trim_transparent_border};
use crate::config::Config;
use crate::job_loaders::{Jobs, LogoJob};
use crate::svg_saver::save_ready_logo;

use futures::future::join_all;
use image::{DynamicImage, GenericImageView, ImageBuffer, ImageReader, Rgb, Rgba};
use indicatif::ProgressBar;
use log::{error, info};
use palette::Srgb;
use std::error::Error;
use std::path::Path;
use std::process::Command;

const WHITE_COLOR: u8 = 250;
const MIN_SCORE_DOMINANT_COLOR: f32 = 0.5;
const GRAY_BACKGROUND_COLOR: Srgb<u8> = Srgb::new(238, 237, 241);
pub async fn remove_border_parallel(
    jobs: Jobs,
    config: &Config,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut tasks = Vec::new();
    let download_folder = config.download_folder();
    let crop_folder = config.crop_folder();

    for logo in jobs.logos.clone() {
        let download_folder = download_folder.clone();
        let crop_folder = crop_folder.clone();
        tasks.push(tokio::spawn(async move {
            let result = remove_border(logo, &download_folder, &crop_folder).await;
            result
        }));
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
    info!("Обработка краев завершена");
    Ok(())
}

async fn remove_border(
    logo: LogoJob,
    download_folder: &Path,
    crop_folder: &Path,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    const BORDER_SIZE: u32 = 1;

    let id = logo.id;
    let small_image_name = download_folder.join(format!("{}.jpg", id));
    let small_rgb_image = load_image(&small_image_name)?;
    let (w, h) = small_rgb_image.dimensions();
    if w >= (BORDER_SIZE + 1) && h >= (BORDER_SIZE + 1) {
        let output_path = crop_folder.join(format!("{}.jpg", id));
        small_rgb_image
            .crop_imm(
                BORDER_SIZE,
                BORDER_SIZE,
                w - BORDER_SIZE * 2,
                h - BORDER_SIZE * 2,
            )
            .save(&output_path)?;
    }
    Ok(())
}

pub async fn images_works_parallel(
    jobs: Jobs,
    config: &Config,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let bar = ProgressBar::new(jobs.logos.len() as u64);

    let mut tasks = Vec::new();
    let mut task_id = 0;
    let download_folder = config.download_folder();
    let upscale_folder = config.upscale_folder();
    let result_folder = config.result_folder();

    for logo in jobs.logos {
        let bar_clone = bar.clone();
        let download_folder = download_folder.clone();
        let upscale_folder = upscale_folder.clone();
        let result_folder = result_folder.clone();

        tasks.push(tokio::spawn(async move {
            let result = process_single_logo(
                logo,
                task_id,
                &download_folder,
                &upscale_folder,
                &result_folder,
            )
            .await;
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

async fn process_single_logo(
    logo: LogoJob,
    task: i32,
    download_folder: &Path,
    upscale_folder: &Path,
    result_folder: &Path,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let id = logo.id;

    let small_image_name = download_folder.join(format!("{}.jpg", id));
    let big_image_name = upscale_folder.join(format!("{}.png", id));

    info!(
        "{} Таска обработки начата. Задача:{} Файлы для обработки: {} {}",
        task,
        id,
        small_image_name.display(),
        big_image_name.display()
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
        final_image = trim_transparent_border(&mut final_image);
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
    let new_image_name = result_folder.join(format!("{}.svg", id));
    let percent = (background.score * 100.0) as u16;

    // Логирование
    info!(
        "{} Доминирующий цвет: RGB({} {} {})  Всего: {}%",
        small_image_name.display(),
        background.color.red,
        background.color.green,
        background.color.blue,
        percent
    );

    // Создание SVG
    let image_path_str = new_image_name.to_str().ok_or("Invalid path")?;
    let _ = save_ready_logo(final_image, id, background, image_path_str, true);

    info!(
        "{} Таска закончена. Задача:{} Файлы для обработки: {} {} Сохранение {}",
        task,
        id,
        small_image_name.display(),
        big_image_name.display(),
        new_image_name.display()
    );
    Ok(())
}

/// Расширения форматов, поддерживаемых ImageReader при включённых фичах крейта image.
const IMAGE_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "bmp", "ico", "tiff", "tif", "tga", "qoi",
    "pnm", "pbm", "pgm", "ppm", "pam", "exr", "hdr", "dds", "ff",
];

/// Загружает изображение из файла. В `image_name` передаётся путь без расширения —
/// функция ищет файл с подходящим расширением (png, jpg, gif, webp и др.) и загружает его.
/// Если в пути уже есть расширение и файл существует, используется он.
fn load_image(image_name: &Path) -> Result<DynamicImage, Box<dyn Error + Send + Sync>> {
    let path_to_open = if image_name.exists() {
        image_name.to_path_buf()
    } else {
        // Путь без расширения или файл не найден — ищем по расширениям
        let base = match image_name.extension() {
            Some(_) => image_name.parent().unwrap_or(image_name).join(
                image_name.file_stem().unwrap_or_default(),
            ),
            None => image_name.to_path_buf(),
        };
        IMAGE_EXTENSIONS
            .iter()
            .map(|ext| base.with_extension(ext))
            .find(|p| p.exists())
            .ok_or_else(|| {
                let msg = format!(
                    "Файл не найден: {} (и варианты с расширениями: {})",
                    image_name.display(),
                    IMAGE_EXTENSIONS[..8].join(", ")
                );
                error!("{}", msg);
                msg
            })?
    };

    let image = ImageReader::open(&path_to_open)?
        .with_guessed_format()?
        .decode()?;

    let image = flatten_alpha_channel(image);
    let (width, height) = image.dimensions();

    info!(
        "Загружена картинка: {} Размер {}x{}",
        path_to_open.display(),
        width,
        height
    );
    Ok(image)
}

fn flatten_alpha_channel(image: DynamicImage) -> DynamicImage {
    match image {
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
    }
}

pub async fn upscale_images(config: &Config) -> Result<(), Box<dyn Error + Send + Sync>> {
    // Usage: upscayl-bin -i infile -o outfile [options]...
    //
    //     -h                   show this help
    //     -i input-path        input image path (jpg/png/webp) or directory
    //     -o output-path       output image path (jpg/png/webp) or directory
    //     -z model-scale       scale according to the model (can be 2, 3, 4. default=4)
    //     -s output-scale      custom output scale (can be 2, 3, 4. default=4)
    //     -r resize            output to dimension (default=WxH:default), use '-r help' for more details
    //     -w width             resize output to a width (default=W:default), use '-r help' for more details
    //     -c compress          compression of the output image, default 0 and varies to 100
    //     -t tile-size         tile size (>=32/0=auto, default=0) can be 0,0,0 for multi-gpu
    //     -m model-path        folder path to the pre-trained models. default=models
    //     -n model-name        model name (default=realesrgan-x4plus, can be realesr-animevideov3 | realesrgan-x4plus-anime | realesrnet-x4plus or any other model)
    //     -g gpu-id            gpu device to use (default=auto) can be 0,1,2 for multi-gpu
    //     -j load:proc:save    thread count for load/proc/save (default=1:2:2) can be 1:2,2,2:2 for multi-gpu
    //     -x                   enable tta mode
    //     -f format            output image format (jpg/png/webp, default=ext/png)
    //     -v                   verbose output

    let input_path = config.crop_folder();
    let output_path = config.upscale_folder();

    const SCALE: usize = 4;
    const COMPRESSION: usize = 100;
    const TYPE: &str = "png";

    let status = Command::new(config.upscayl_bin())
        .arg("-i")
        .arg(input_path.to_str().ok_or("Invalid UTF-8 in input path")?)
        .arg("-o")
        .arg(output_path.to_str().ok_or("Invalid UTF-8 in output path")?)
        .arg("-m")
        .arg(config.upscayl_models())
        .arg("-n")
        .arg(config.upscayl_model())
        .arg("-s")
        .arg(SCALE.to_string())
        .arg("-f")
        .arg(TYPE)
        .arg("-v")
        .arg("-c")
        .arg(COMPRESSION.to_string())
        .status()?;

    if !status.success() {
        return Err(format!("Upscayl failed for {}", input_path.display()).into());
    }

    info!("✅ Completed: {}", output_path.display());

    Ok(())
}
