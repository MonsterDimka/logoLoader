use image::RgbaImage;
use visioncortex::{ColorImage, PathSimplifyMode, PointF64};
use vtracer::Config;

pub fn image_vectorize_to_svg(rgba_img: &RgbaImage) -> Result<String, String> {
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
        Err(_) => return Err(String::from("Не могу векторизовать SVG")),
    };

    let mut svg_string = String::new();

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

    // Ok(out_svg.to_string())
    Ok(svg_string)
}
