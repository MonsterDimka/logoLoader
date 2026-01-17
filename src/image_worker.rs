use crate::{DOWNLOAD_FOLDER, LogoJob, RESULT_FOLDER, UPSCALE_FOLDER};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use futures::future::join_all;
use image::{DynamicImage, GenericImageView, ImageReader, RgbImage, RgbaImage};
use indicatif::ProgressBar;
use kmeans_colors::{Sort, get_kmeans};
use log::{error, info};
use oxipng::{Options, optimize_from_memory};
use palette::cast::from_component_slice;
use palette::{IntoColor, Lab, Srgb};
use std::error::Error;
use std::path::Path;
use visioncortex::{ColorImage, PathSimplifyMode, PointF64};
use vtracer::Config;

const WIDTH_HEIGHT: usize = 300;
const MAX_VECTOR_LOGO_SIZE: usize = 90;

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
    let gray_background_color = Srgb::new(238, 237, 241);
    let id = logo.id;

    // Загружаем изображения
    let small_image_name = format!("{}/{}.jpg", DOWNLOAD_FOLDER, id);
    let big_image_name = format!("{}/{}.png", UPSCALE_FOLDER, id);

    info!(
        "{} Таска обработки начата. Задача:{} Файлы для обработки: {} {}",
        task, id, small_image_name, big_image_name
    );

    let small_rgb_image = load_image(&small_image_name)?.to_rgb8();
    let big_rgba_image = load_image(&big_image_name)?.to_rgba8();
    let mut processed_image = big_rgba_image;

    // Анализ доминирующего цвета
    let (color, score, average) = dominant_colors(small_rgb_image.clone())?;

    // Обработка фона
    if score > 0.5 {
        remove_image_background(color, &mut processed_image);
    }

    // Выбор цвета фона
    let background_color = if average > 250 {
        gray_background_color
    } else {
        color
    };

    // Формирование имени SVG файла
    let percent = (score * 100.0) as u16;
    let new_image_name = format!("{}/{}.svg", RESULT_FOLDER, id);

    // Логирование
    info!(
        "{} Доминирующий цвет: RGB({} {} {})  Всего: {}%",
        small_image_name, color.red, color.green, color.blue, percent
    );

    // Создание SVG
    let _ = save_ready_logo(processed_image, id, background_color, &new_image_name, true);

    info!(
        "{} Таска закончена. Задача:{} Файлы для обработки: {} {} Cохранение {}",
        task, id, small_image_name, big_image_name, new_image_name
    );
    Ok(())
}

fn remove_image_background(color: Srgb<u8>, big_rgba_image: &mut RgbaImage) {
    big_rgba_image.pixels_mut().for_each(|p| {
        let alpha = if in_range(p[0], color.red, 30)
            && in_range(p[1], color.green, 30)
            && in_range(p[2], color.blue, 30)
        {
            0 // Почти белые пиксели прозрачны
        } else {
            u8::MAX
        };
        p[3] = alpha;
    });
}

const KILOBYTE: usize = 1024;
const PNG_OPTIMIZE: u8 = 4;

fn save_ready_logo(
    image: RgbaImage,
    job_id: u32,
    background_color: Srgb<u8>,
    image_name: &str,
    optimize: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let dimage = DynamicImage::ImageRgba8(image.clone());
    let output_path = Path::new(&image_name);
    // Конвертируем изображение в PNG bytes
    let mut png_bytes = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut png_bytes);
    dimage.write_to(&mut cursor, image::ImageFormat::Png)?;

    // Векторизуем
    let svg_logo = image_vectorize_to_svg(&image)?;

    // Если векторизация большого размера используем PNG
    let result = if svg_logo.len() / KILOBYTE < MAX_VECTOR_LOGO_SIZE {
        format!(
            r#"<!-- Curved SVG  -->
            {svg_logo}"#
        )
    } else {
        // Кодируем в base64
        let base64_image = if optimize {
            // Optimize the PNG data in memory
            let options = Options::from_preset(PNG_OPTIMIZE);
            let optimized_png_data = optimize_from_memory(&png_bytes, &options)?;
            BASE64.encode(&optimized_png_data)
        } else {
            BASE64.encode(&png_bytes)
        };

        format!(
            r#"<!-- PNG RGBA as Base64 -->
    <image width="100%" height="100%" id="logo_image"
           preserveAspectRatio="xMidYMid meet"
           xlink:href="data:image/png;base64,{base64_image}"/>"#
        )
    };

    // Создаем SVG
    let svg_content = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg width="{WIDTH_HEIGHT}" height="{WIDTH_HEIGHT}"
     xmlns="http://www.w3.org/2000/svg"
     xmlns:xlink="http://www.w3.org/1999/xlink">
    <title>{job_id}</title>
    <!-- Background -->
    <rect width="100%" height="100%" id="Задник"
          fill="rgb({r},{g},{b})"/>

    <!-- Logo -->
    {result}
</svg>"#,
        r = background_color.red,
        g = background_color.green,
        b = background_color.blue,
    );

    // Сохраняем файл
    info!("Сохраняем файл {}", output_path.to_str().unwrap());
    // println!("{} Размер svg {} Кбайт Картинка {} Кбайт", output_path.to_str().unwrap(), svg_content.len() / 1024, base64_image.len() / 1024);

    std::fs::write(output_path, svg_content)?;

    Ok(())
}

/// Попадает ли цвет в доминирующий
fn in_range(value: u8, dominant: u8, tolerance: u8) -> bool {
    let lower_bound = dominant.saturating_sub(tolerance);
    let upper_bound = dominant.saturating_add(tolerance);
    value >= lower_bound && value <= upper_bound
}

/// Вычисление доминирующего цвета
fn dominant_colors(rgb_img: RgbImage) -> Result<(Srgb<u8>, f32, u8), Box<dyn Error + Send + Sync>> {
    // Конвертация в Lab для лучшей кластеризации
    let lab_pixels: Vec<Lab> = from_component_slice::<Srgb<u8>>(&rgb_img)
        .iter()
        .map(|&srgb| srgb.into_linear().into_color())
        .collect();

    // Параметры кластеризации
    const K: usize = 5;
    const MAX_ITER: usize = 100;
    const CONVERGE: f32 = 1.0;

    // K-means кластеризация
    let result = get_kmeans(K, MAX_ITER, CONVERGE, false, &lab_pixels, 0);

    // Сортировка по доминированию
    let mut colors = Lab::sort_indexed_colors(&result.centroids, &result.indices);
    colors.sort_unstable_by(|a, b| b.percentage.total_cmp(&a.percentage));

    // Извлечение доминантного цвета
    let dominant = colors.first().ok_or("No clusters found")?;
    let dominant_rgb = Srgb::from_linear(dominant.centroid.into_color());
    let dominant_color_average = ((f64::from(dominant_rgb.red)
        + f64::from(dominant_rgb.green)
        + f64::from(dominant_rgb.blue))
        / 3.0)
        .round() as u8;

    Ok((dominant_rgb, dominant.percentage, dominant_color_average))
}

fn load_image(image_name: &str) -> Result<DynamicImage, Box<dyn Error + Send + Sync>> {
    // Проверка существования файла
    if !Path::new(image_name).exists() {
        error!("Файл не найден: {}", image_name);
    }

    let image = ImageReader::open(image_name)?
        .with_guessed_format()?
        .decode()?;
    let (imgx, imgy) = image.dimensions();

    // Логирование
    info!(
        "Загружена картинка: {} Размер картинки {}x{} формат",
        image_name, imgx, imgy
    );
    Ok(image)
}

fn image_vectorize_to_svg(rgba_img: &RgbaImage) -> Result<String, String> {
    let convert_config = Config {
        color_mode: vtracer::ColorMode::Color, // or another ColorMode variant
        hierarchical: vtracer::Hierarchical::Stacked, // or another Hierarchical variant
        filter_speckle: 16,
        color_precision: 5,
        layer_difference: 16,
        mode: PathSimplifyMode::Spline,
        corner_threshold: 60,
        length_threshold: 4.0,
        max_iterations: 10,
        splice_threshold: 45,
        path_precision: Some(4),
    };

    let (width, height) = (rgba_img.width() as usize, rgba_img.height() as usize);
    let color_image = ColorImage {
        pixels: rgba_img.as_raw().to_vec(),
        width,
        height,
    };

    // Convert the image to SVG
    let svg = vtracer::convert(color_image, convert_config);

    let out_svg = match svg {
        Ok(file) => file,
        Err(_) => return Err(String::from("Cannot create output file.")),
    };

    let scale_x = WIDTH_HEIGHT as f64 / width as f64;
    let scale_y = WIDTH_HEIGHT as f64 / height as f64;
    let scale = scale_x.max(scale_y);

    let mut svg_string = String::new();
    svg_string.push_str(&format!(r#"<g transform="scale({:.2})">"#, scale));

    for svg_path in &out_svg.paths {
        let (string, offset) =
            svg_path
                .path
                .to_svg_string(true, PointF64::default(), out_svg.path_precision);
        svg_string.push_str(&format!(
            "<path d=\"{}\" fill=\"{}\" transform=\"translate({},{})\"/>",
            string,
            svg_path.color.to_hex_string(),
            offset.x,
            offset.y
        ));
    }

    svg_string.push_str("</g>");

    // Ok(out_svg.to_string())
    Ok(svg_string)
}
