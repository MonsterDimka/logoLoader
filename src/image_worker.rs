use crate::background_works::{DominantColor, trim_transparent_border};
use crate::config::{CROP_FOLDER, DOWNLOAD_FOLDER, RESULT_FOLDER, UPSCALE_FOLDER};
use crate::job_loaders::{Jobs, LogoJob};
use crate::save::save_ready_logo;

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
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut tasks = Vec::new();

    for logo in jobs.logos.clone() {
        tasks.push(tokio::spawn(async move {
            let result = remove_border(logo).await;
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

async fn remove_border(logo: LogoJob) -> Result<(), Box<dyn Error + Send + Sync>> {
    const BORDER_SIZE: u32 = 1;

    let id = logo.id;
    let small_image_name = format!("{}/{}.jpg", DOWNLOAD_FOLDER, id);
    let small_rgb_image = load_image(&small_image_name)?;
    let (w, h) = small_rgb_image.dimensions();
    if w >= (BORDER_SIZE + 1) && h >= (BORDER_SIZE + 1) {
        small_rgb_image
            .crop_imm(
                BORDER_SIZE,
                BORDER_SIZE,
                w - BORDER_SIZE * 2,
                h - BORDER_SIZE * 2,
            )
            .save(format!("{}/{}.jpg", CROP_FOLDER, id))?;
    }
    Ok(())
}

pub async fn images_works_parallel(jobs: Jobs) -> Result<(), Box<dyn Error + Send + Sync>> {
    let bar = ProgressBar::new(jobs.logos.len() as u64);

    let mut tasks = Vec::new();
    let mut task_id = 0;

    for logo in jobs.logos {
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
    let _ = save_ready_logo(final_image, id, background, &new_image_name, true);

    info!(
        "{} Таска закончена. Задача:{} Файлы для обработки: {} {} Сохранение {}",
        task, id, small_image_name, big_image_name, new_image_name
    );
    Ok(())
}

fn load_image(image_name: &str) -> Result<DynamicImage, Box<dyn Error + Send + Sync>> {
    // Проверка существования файла
    if !Path::new(image_name).exists() {
        println!("Файл не найден: {}", image_name);
        error!("Файл не найден: {}", image_name);
    }

    let image = ImageReader::open(image_name)?
        .with_guessed_format()?
        .decode()?;

    // Проверяем, есть ли альфа-канал и конвертируем в RGB если нужно
    let image = flatten_alpha_channel(image);

    let (width, height) = image.dimensions();

    // Логирование
    info!("Загружена картинка: {image_name} Размер картинки {width}x{height} формат");
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

pub async fn upscale_images() -> Result<(), Box<dyn Error + Send + Sync>> {
    // Usage: upscayl-bin -i infile -o outfile [options]...
    //
    //     -h                   show this help
    //     -i input-path        input image path (jpg/png/webp) or directory
    //     -o output-path       output image path (jpg/png/webp) or directory
    //     -z model-scale       scale according to the model (can be 2, 3, 4. default=4)
    //     -s output-scale      custom output scale (can be 2, 3, 4. default=4)
    //     -r resize            resize output to dimension (default=WxH:default), use '-r help' for more details
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

    const BASE_PATH: &str = "/Users/kapustindmitri/RustroverProjects/logoLoader/";
    let input_path = Path::new(BASE_PATH).join(CROP_FOLDER);
    let output_path = Path::new(BASE_PATH).join(UPSCALE_FOLDER);

    const UPSCALER_PROG: &str = "/Applications/Upscayl.app/Contents/Resources/bin/upscayl-bin";
    const MODEL_PATH: &str = "/Applications/Upscayl.app/Contents/Resources/models";
    const MODEL_NAME: &str = "upscayl-standard-4x";
    const SCALE: usize = 4;
    const COMPRESSION: usize = 100;
    const TYPE: &str = "png";

    let status = Command::new(UPSCALER_PROG)
        .arg("-i")
        .arg(input_path.to_str().expect("Invalid UTF-8 in input path"))
        .arg("-o")
        .arg(output_path.to_str().expect("Invalid UTF-8 in input path"))
        .arg("-m")
        .arg(&MODEL_PATH)
        .arg("-n")
        .arg(&MODEL_NAME)
        .arg("-s")
        .arg(SCALE.to_string())
        .arg("-f")
        .arg(&TYPE)
        .arg("-v")
        .arg("-c")
        .arg(COMPRESSION.to_string())
        .status()
        .expect("failed to execute process");

    if !status.success() {
        return Err(format!("Upscayl failed for {}", input_path.to_str().expect("err")).into());
    }

    info!("✅ Completed: {}", output_path.to_str().expect("err"));

    Ok(())
}
