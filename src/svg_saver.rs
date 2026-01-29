use crate::background_works::DominantColor;
use crate::vectorize;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use image::{DynamicImage, RgbaImage};
use log::info;
use oxipng::{optimize_from_memory, Options};
use std::error::Error;
use std::path::Path;

const MAX_VECTOR_LOGO_SIZE: usize = 100;
const KILOBYTE: usize = 1024;
const PNG_OPTIMIZE: u8 = 4;
const WIDTH_HEIGHT: usize = 300;
const LOGO_SCALE_FACTOR: f64 = 0.65;

const SVG_SHEME: &str = r#"<g id="none-copy-2646" stroke="none" stroke-width="1" fill="none" fill-rule="evenodd">
    <g id="Group" opacity="0.2" stroke="Black">
            <g id="Group">
                <circle id="Oval" cx="150" cy="150" r="149.5"></circle>
                <line x1="0" y1="150" x2="300" y2="150" id="Line" stroke-linecap="square"></line>
                <line x1="0" y1="150" x2="300" y2="150" id="Line" stroke-linecap="square" transform="translate(150, 150) rotate(-270) translate(-150, -150)"></line>
                <line x1="0" y1="150" x2="300" y2="150" id="Line" stroke-linecap="square" transform="translate(150, 150) rotate(-225) translate(-150, -150)"></line>
                <line x1="0" y1="150" x2="300" y2="150" id="Line" stroke-linecap="square" transform="translate(150, 150) rotate(-315) translate(-150, -150)"></line>
                <rect id="Rectangle" x="63.5" y="63.5" width="173" height="173"></rect>
            </g>
        </g>
    </g>"#;

pub fn save_ready_logo(
    image: RgbaImage,
    job_id: u32,
    background_color: DominantColor,
    image_file_name: &str,
    optimize: bool,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let base64_png_logo = make_png_base64(&image, optimize)?;
    let vector_svg_logo = vectorize::image_vectorize_to_svg(&image)?;
    let should_use_vector =
        vector_svg_logo.len() / KILOBYTE < MAX_VECTOR_LOGO_SIZE && background_color.score > 0.5;
    if should_use_vector {
        info!(
            "Для {image_file_name} PNG {} SVG {} score {} выбран SVG {should_use_vector}",
            base64_png_logo.len(),
            vector_svg_logo.len(),
            background_color.score
        )
    } else {
        info!(
            "Для {image_file_name} PNG {} SVG {}  score {} выбран PNG {should_use_vector}",
            base64_png_logo.len(),
            vector_svg_logo.len(),
            background_color.score
        )
    };

    // Если векторизация большого размера используем PNG
    let logo_svg = if should_use_vector {
        let transform = LogoTransform::calculate_transform(&image);

        format!(
            r#"<!-- Curved SVG  -->
<g transform="translate({offset_x:.2}, {offset_y:.2}) scale({scale:.2})">
            {vector_svg_logo}
</g>"#,
            offset_x = transform.offset_x,
            offset_y = transform.offset_y,
            scale = transform.scale
        )
    } else {
        let transform = if background_color.score < 0.5 {
            LogoTransform::full_size()
        } else {
            LogoTransform::calculate_png_transform(&image, image_file_name)
            // (LOGO_SCALE_FACTOR, offset_x, offset_y)
        };
        format!(
            r#"<!-- PNG RGBA as Base64 -->
<g transform="translate({offset_x:.2}, {offset_y:.2}) scale({scale:.2})">
    <image width="100%" height="100%" id="logo_image"
                        preserveAspectRatio="xMidYMid meet"
                        xlink:href="data:image/png;base64,{base64_png_logo}"/>
</g>"#,
            offset_x = transform.offset_x,
            offset_y = transform.offset_y,
            scale = transform.scale
        )
    };

    // Создаем SVG
    let svg_file = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg width="{WIDTH_HEIGHT}" height="{WIDTH_HEIGHT}" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
    <title>{job_id}</title>
    <!-- Background -->
    <rect width="100%" height="100%" id="Задник" fill="rgb({r},{g},{b})"/>
    <!-- Logo -->
    {logo_svg}
</svg>"#,
        r = background_color.color.red,
        g = background_color.color.green,
        b = background_color.color.blue,
    );

    // Сохраняем файл
    let output_path = Path::new(&image_file_name);
    info!("Сохраняем файл {}", output_path.to_str().unwrap());
    std::fs::write(output_path, svg_file)?;

    Ok(())
}

// Вспомогательные структуры для лучшей организации данных
struct LogoTransform {
    scale: f64,
    offset_x: f64,
    offset_y: f64,
}

impl LogoTransform {
    fn calculate_transform(image: &RgbaImage) -> LogoTransform {
        let (w, h) = (image.width() as f64, image.height() as f64);
        let target = WIDTH_HEIGHT as f64;

        let scale = (target / w).min(target / h) * LOGO_SCALE_FACTOR;
        let offset_x = (target - w * scale) / 2.0;
        let offset_y = (target - h * scale) / 2.0;

        LogoTransform {
            scale,
            offset_x,
            offset_y,
        }
    }

    fn calculate_png_transform(image: &RgbaImage, name: &str) -> LogoTransform {
        let (width, height) = (image.width() as f64, image.height() as f64);
        let target = WIDTH_HEIGHT as f64;
        let scale = LOGO_SCALE_FACTOR;

        let scaled_size = target * scale;
        let offset = (target - scaled_size) / 2.0;

        info!(
            r#"Размер картинки {name} подрезанный: {width:.2}x{height:.2}
Сжатие до размера 300x300: {scale_x:.2}x{scale_y:.2} => Итоговое сжатие: {scale:.2}
Сжатый размер: {scaled_size:.2}x{scaled_size:.2}
Смещение к центру: {offset:.2}x{offset:.2}"#,
            scale_x = target / width,
            scale_y = target / height,
        );

        LogoTransform {
            scale,
            offset_x: offset,
            offset_y: offset,
        }
    }
    fn full_size() -> LogoTransform {
        LogoTransform {
            scale: 1.0,
            offset_x: 0.0,
            offset_y: 0.0,
        }
    }
}

fn make_png_base64(image: &RgbaImage, optimize: bool) -> Result<String, Box<dyn Error + Send + Sync>> {
    // Конвертируем изображение в PNG bytes
    let dimage = DynamicImage::ImageRgba8(image.clone());
    let mut png_bytes = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut png_bytes);
    dimage.write_to(&mut cursor, image::ImageFormat::Png)?;

    // Кодируем в base64
    let base64_image = if optimize {
        // Optimize the PNG data in memory
        let options = Options::from_preset(PNG_OPTIMIZE);
        let optimized_png_data = optimize_from_memory(&png_bytes, &options)?;
        BASE64.encode(&optimized_png_data)
    } else {
        BASE64.encode(&png_bytes)
    };
    Ok(base64_image)
}
