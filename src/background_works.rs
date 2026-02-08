use image::{imageops::FilterType, GenericImageView, RgbImage, Rgba, RgbaImage};
use kmeans_colors::{get_kmeans, Sort};
use log::info;
use palette::cast::from_component_slice;
use palette::{IntoColor, Lab, Srgb, Srgba};
use std::error::Error;
#[derive(Debug)]
pub struct DominantColor {
    pub color: Srgb<u8>,
    pub score: f32,
    pub average: u8,
    pub k: usize,
}

impl DominantColor {
    pub fn white() -> Self {
        DominantColor {
            color: Srgb::new(255, 255, 255),
            score: 1.0,
            average: 255,
            k: 6,
        }
    }
    pub fn remove_image_background(&self, big_rgba_image: &mut RgbaImage) {
        const TOLERANCE: u8 = 30;
        let (r, g, b) = (self.color.red, self.color.green, self.color.blue);

        // Используем прямой доступ к данным
        let pixels = big_rgba_image.as_mut();
        for pixel in pixels.chunks_exact_mut(4) {
            pixel[3] = if pixel[0].abs_diff(r) <= TOLERANCE
                && pixel[1].abs_diff(g) <= TOLERANCE
                && pixel[2].abs_diff(b) <= TOLERANCE
            {
                0
            } else {
                255
            };
        }
    }

    /// Вычисление доминирующего цвета
    pub fn from_rgba_image(
        rgb_img: RgbImage,
    ) -> Result<DominantColor, Box<dyn Error + Send + Sync>> {
        let (w, h) = rgb_img.dimensions();

        // Конвертация в Lab для лучшей кластеризации
        let lab_pixels: Vec<Lab> = from_component_slice::<Srgb<u8>>(&rgb_img)
            .iter()
            .map(|&srgb| srgb.into_linear().into_color())
            .collect();

        // Параметры кластеризации
        // const K: usize = 5;
        // Подбор K: больше пикселей — больше кластеров, в разумных пределах 3..=8
        let k = (3 + (lab_pixels.len() / 9000).min(5)).clamp(4, 6);
        const MAX_ITER: usize = 100;
        const CONVERGE: f32 = 1.0;

        // K-means кластеризация
        let result = get_kmeans(k, MAX_ITER, CONVERGE, false, &lab_pixels, 0);

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
            k: k,
        })
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
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
    }

    // Все пиксели прозрачные
    if min_x > max_x || min_y > max_y {
        return RgbaImage::from_pixel(1, 1, Rgba([0, 0, 0, 0]));
    }

    // Создаем прямоугольник для обрезки
    let w = max_x - min_x + 1;
    let h = max_y - min_y + 1;
    let cropped_view = image.view(min_x, min_y, w, h);

    info!(
        "Обрезка прозрачных краев: {}x{} at ({}, {})",
        w, h, min_x, min_y
    );

    cropped_view.to_image()
}
