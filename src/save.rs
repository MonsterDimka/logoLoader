use crate::background_works::DominantColor;
use crate::vectorize;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use image::{DynamicImage, RgbaImage};
use log::info;
use oxipng::{Options, optimize_from_memory};
use std::error::Error;
use std::path::Path;

const MAX_VECTOR_LOGO_SIZE: usize = 90;
const KILOBYTE: usize = 1024;
const PNG_OPTIMIZE: u8 = 0;
const WIDTH_HEIGHT: usize = 300;
const LOGO_SCALE_FACTOR: f64 = 0.7;

pub fn save_ready_logo(
    image: RgbaImage,
    job_id: u32,
    background_color: DominantColor,
    image_file_name: &str,
    optimize: bool,
) -> Result<(), Box<dyn Error>> {
    let base64_png_logo = make_png_base64(&image, optimize)?;
    let vector_svg_logo = vectorize::image_vectorize_to_svg(&image)?;

    let (width, height) = (image.width() as usize, image.height() as usize);

    let scale_x = WIDTH_HEIGHT as f64 / width as f64;
    let scale_y = WIDTH_HEIGHT as f64 / height as f64;
    let mut scale = scale_x.min(scale_y) * LOGO_SCALE_FACTOR;

    let scaled_width = width as f64 * scale;
    let scaled_height = height as f64 * scale;

    // let mut scaled_width = (width as f64 * LOGO_SCALE_FACTOR) as i32;
    // let mut scaled_height = (height as f64 * LOGO_SCALE_FACTOR) as i32;
    let mut offset_x = (WIDTH_HEIGHT as f64 - scaled_width) / 2.0;
    let mut offset_y = (WIDTH_HEIGHT as f64 - scaled_height) / 2.0;

    println!(
        "Размер подрезанный: {} {}  Сжатый размер: {:.2} {:.2}  Смещение: {:.2} {:.2} Итоговое сжатие: {:.2}",
        width, height, scaled_width, scaled_height, offset_x, offset_y, scale
    );

    // Если векторизация большого размера используем PNG
    let logo_svg = if vector_svg_logo.len() / KILOBYTE < MAX_VECTOR_LOGO_SIZE {
        format!(
            r#"<!-- Curved SVG  -->
            <g transform="translate({offset_x:.2}, {offset_y:.2}) scale({scale:.2})">
            {vector_svg_logo}</g>"#
        )
    } else {
        (scale, offset_x, offset_y) = if background_color.score < 0.5 {
            (1.0, 0.0, 0.0)
        } else {
            (LOGO_SCALE_FACTOR, offset_x, offset_y)
        };
        format!(
            r#"<!-- PNG RGBA as Base64 -->
            <g transform="translate({offset_x:.2}, {offset_y:.2}) scale({scale:.2})">
    <image width="100%" height="100%" id="logo_image"
           preserveAspectRatio="xMidYMid meet"
           xlink:href="data:image/png;base64,{base64_png_logo}"/></g>"#
        )
    };

    // Создаем SVG
    let svg_file = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg width="{WIDTH_HEIGHT}" height="{WIDTH_HEIGHT}"
     xmlns="http://www.w3.org/2000/svg"
     xmlns:xlink="http://www.w3.org/1999/xlink">
    <title>{job_id}</title>
    <!-- Background -->
    <rect width="100%" height="100%" id="Задник"
          fill="rgb({r},{g},{b})"/>

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

fn make_png_base64(image: &RgbaImage, optimize: bool) -> Result<String, Box<dyn Error>> {
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
