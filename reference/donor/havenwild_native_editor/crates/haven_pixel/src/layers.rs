use crate::{PixelBlendMode, PixelLayer};
use image::{Rgba, RgbaImage};

pub(crate) fn composite_layers(layers: &[PixelLayer], width: u32, height: u32) -> RgbaImage {
    let mut output = RgbaImage::from_pixel(width, height, Rgba([0, 0, 0, 0]));
    for y in 0..height {
        for x in 0..width {
            output.put_pixel(x, y, Rgba(composite_pixel(layers, x, y)));
        }
    }
    output
}

pub(crate) fn composite_pixel(layers: &[PixelLayer], x: u32, y: u32) -> [u8; 4] {
    let mut destination = [0, 0, 0, 0];
    for layer in layers {
        if !layer.metadata.visible || x >= layer.image.width() || y >= layer.image.height() {
            continue;
        }
        destination = blend(
            destination,
            layer.image.get_pixel(x, y).0,
            layer.metadata.opacity,
            layer.metadata.blend_mode,
        );
    }
    destination
}

pub(crate) fn flatten_pair(bottom: &PixelLayer, top: &PixelLayer) -> RgbaImage {
    let width = bottom.image.width().max(top.image.width());
    let height = bottom.image.height().max(top.image.height());
    let mut output = RgbaImage::from_pixel(width, height, Rgba([0, 0, 0, 0]));
    for y in 0..height {
        for x in 0..width {
            let bottom_pixel = if x < bottom.image.width() && y < bottom.image.height() {
                bottom.image.get_pixel(x, y).0
            } else {
                [0, 0, 0, 0]
            };
            let top_pixel = if x < top.image.width() && y < top.image.height() {
                top.image.get_pixel(x, y).0
            } else {
                [0, 0, 0, 0]
            };
            let base = blend(
                [0, 0, 0, 0],
                bottom_pixel,
                bottom.metadata.opacity,
                bottom.metadata.blend_mode,
            );
            output.put_pixel(
                x,
                y,
                Rgba(blend(
                    base,
                    top_pixel,
                    top.metadata.opacity,
                    top.metadata.blend_mode,
                )),
            );
        }
    }
    output
}

fn blend(destination: [u8; 4], source: [u8; 4], opacity: u8, mode: PixelBlendMode) -> [u8; 4] {
    let opacity = opacity as f32 / 255.0;
    let source_alpha = source[3] as f32 / 255.0 * opacity;
    let destination_alpha = destination[3] as f32 / 255.0;

    if mode == PixelBlendMode::Erase {
        let output_alpha = destination_alpha * (1.0 - source_alpha);
        if output_alpha <= f32::EPSILON {
            return [0, 0, 0, 0];
        }
        return [
            destination[0],
            destination[1],
            destination[2],
            to_byte(output_alpha),
        ];
    }
    if source_alpha <= f32::EPSILON {
        return destination;
    }

    let source_rgb = [
        source[0] as f32 / 255.0,
        source[1] as f32 / 255.0,
        source[2] as f32 / 255.0,
    ];
    let destination_rgb = [
        destination[0] as f32 / 255.0,
        destination[1] as f32 / 255.0,
        destination[2] as f32 / 255.0,
    ];
    let blended = match mode {
        PixelBlendMode::Normal => source_rgb,
        PixelBlendMode::Multiply => [
            source_rgb[0] * destination_rgb[0],
            source_rgb[1] * destination_rgb[1],
            source_rgb[2] * destination_rgb[2],
        ],
        PixelBlendMode::Screen => [
            1.0 - (1.0 - source_rgb[0]) * (1.0 - destination_rgb[0]),
            1.0 - (1.0 - source_rgb[1]) * (1.0 - destination_rgb[1]),
            1.0 - (1.0 - source_rgb[2]) * (1.0 - destination_rgb[2]),
        ],
        PixelBlendMode::Add => [
            (source_rgb[0] + destination_rgb[0]).min(1.0),
            (source_rgb[1] + destination_rgb[1]).min(1.0),
            (source_rgb[2] + destination_rgb[2]).min(1.0),
        ],
        PixelBlendMode::Erase => unreachable!("erase handled before color blending"),
    };

    let output_alpha = source_alpha + destination_alpha * (1.0 - source_alpha);
    if output_alpha <= f32::EPSILON {
        return [0, 0, 0, 0];
    }
    let mut output = [0u8; 4];
    for channel in 0..3 {
        let premultiplied = blended[channel] * source_alpha
            + destination_rgb[channel] * destination_alpha * (1.0 - source_alpha);
        output[channel] = to_byte(premultiplied / output_alpha);
    }
    output[3] = to_byte(output_alpha);
    output
}

fn to_byte(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PixelLayer, PixelLayerMetadata};

    fn layer(name: &str, color: [u8; 4]) -> PixelLayer {
        PixelLayer {
            metadata: PixelLayerMetadata::new(name, name),
            image: RgbaImage::from_pixel(1, 1, Rgba(color)),
        }
    }

    #[test]
    fn normal_layers_composite_in_order() {
        let bottom = layer("bottom", [255, 0, 0, 255]);
        let top = layer("top", [0, 0, 255, 128]);
        let pixel = composite_pixel(&[bottom, top], 0, 0);
        assert!(pixel[0] >= 126 && pixel[0] <= 129);
        assert!(pixel[2] >= 126 && pixel[2] <= 129);
        assert_eq!(pixel[3], 255);
    }

    #[test]
    fn hidden_layers_do_not_affect_output() {
        let bottom = layer("bottom", [20, 30, 40, 255]);
        let mut top = layer("top", [240, 10, 10, 255]);
        top.metadata.visible = false;
        assert_eq!(composite_pixel(&[bottom, top], 0, 0), [20, 30, 40, 255]);
    }
}
