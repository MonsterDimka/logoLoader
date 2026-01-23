use image::{DynamicImage, GenericImageView, RgbImage, Rgba, RgbaImage};
use imageproc::rect::Rect;
use kmeans_colors::{Sort, get_kmeans};
use log::info;
use palette::cast::from_component_slice;
use palette::{IntoColor, Lab, Srgb};
use std::error::Error;
#[derive(Debug)]
pub struct DominantColor {
    pub color: Srgb<u8>,
    pub score: f32,
    pub average: u8,
}

impl DominantColor {
    pub fn remove_image_background(&self, big_rgba_image: &mut RgbaImage) {
        const TOLERANCE: u8 = 30;
        const NO_TRANSPARENCY: u8 = 255;
        const TRANSPARENCY: u8 = 0;

        let r = self.color.red;
        let g = self.color.green;
        let b = self.color.blue;

        // Предвычисляем границы
        let (r_low, r_high) = (r.saturating_sub(TOLERANCE), r.saturating_add(TOLERANCE));
        let (g_low, g_high) = (g.saturating_sub(TOLERANCE), g.saturating_add(TOLERANCE));
        let (b_low, b_high) = (b.saturating_sub(TOLERANCE), b.saturating_add(TOLERANCE));

        // Используем прямой доступ к данным
        let pixels = big_rgba_image.as_mut();
        for pixel in pixels.chunks_exact_mut(4) {
            let is_in_range = pixel[0] >= r_low
                && pixel[0] <= r_high
                && pixel[1] >= g_low
                && pixel[1] <= g_high
                && pixel[2] >= b_low
                && pixel[2] <= b_high;
            pixel[3] = if is_in_range {
                TRANSPARENCY
            } else {
                NO_TRANSPARENCY
            };
        }
    }

    /// Вычисление доминирующего цвета
    pub fn from_rgb_image(
        rgb_img: RgbImage,
    ) -> Result<DominantColor, Box<dyn Error + Send + Sync>> {
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

        Ok(DominantColor {
            color: dominant_rgb,
            score: dominant.percentage,
            average: dominant_color_average,
        })
    }

    pub fn from_rgba_image(
        rgba_img: RgbaImage,
    ) -> Result<DominantColor, Box<dyn Error + Send + Sync>> {
        let dynamic = DynamicImage::ImageRgba8(rgba_img);
        let rgb_dynamic = dynamic.into_rgb8();

        return DominantColor::from_rgb_image(rgb_dynamic);
    }
}

// Обрезка прозрачных краев изображения
pub fn trim_transparent_border(image: &RgbaImage) -> RgbaImage {
    let (width, height) = image.dimensions();
    let mut min_x = width;
    let mut max_x = 0;
    let mut min_y = height;
    let mut max_y = 0;

    // Находим bounding box непрозрачных пикселей
    for (x, y, pixel) in image.enumerate_pixels() {
        if pixel[3] != 0 {
            if x < min_x {
                min_x = x;
            }
            if x > max_x {
                max_x = x;
            }
            if y < min_y {
                min_y = y;
            }
            if y > max_y {
                max_y = y;
            }
        }
    }

    // Все пиксели прозрачные
    if min_x > max_x || min_y > max_y {
        return RgbaImage::from_pixel(1, 1, Rgba([0, 0, 0, 0]));
    }

    // Создаем прямоугольник для обрезки
    let rect = Rect::at(min_x as i32, min_y as i32).of_size(max_x - min_x + 1, max_y - min_y + 1);

    // Используем view для получения обрезанного изображения
    let cropped_view = image.view(
        rect.left() as u32,
        rect.top() as u32,
        rect.width(),
        rect.height(),
    );

    info!("Обрезка прозрачных краев изображения: {:?}", rect);

    cropped_view.to_image()
}
