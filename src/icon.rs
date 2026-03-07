pub fn icon_rgba() -> Vec<u8> {
    let width = 32u32;
    let height = 32u32;
    let mut rgba = vec![0u8; (width * height * 4) as usize];

    let bg_color = (40, 44, 52, 255);
    let fg_color = (97, 175, 239, 255);
    let accent_color = (152, 195, 121, 255);

    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;
            let cx = x as i32 - 16;
            let cy = y as i32 - 16;
            let dist = ((cx * cx + cy * cy) as f32).sqrt();

            if dist < 14.0 {
                rgba[idx] = bg_color.0;
                rgba[idx + 1] = bg_color.1;
                rgba[idx + 2] = bg_color.2;
                rgba[idx + 3] = bg_color.3;

                let is_center = cx.abs() < 3 && cy.abs() < 6;
                let is_left_btn = (cx + 6).abs() < 2 && (cy - 3).abs() < 2;
                let is_right_btn = (cx - 6).abs() < 2 && (cy - 3).abs() < 2;
                let is_left_analog = (cx + 5).abs() < 3 && (cy + 3).abs() < 3;
                let is_right_analog = (cx - 5).abs() < 3 && (cy + 3).abs() < 3;

                if is_center || is_left_btn || is_right_btn {
                    rgba[idx] = fg_color.0;
                    rgba[idx + 1] = fg_color.1;
                    rgba[idx + 2] = fg_color.2;
                    rgba[idx + 3] = fg_color.3;
                } else if is_left_analog || is_right_analog {
                    rgba[idx] = accent_color.0;
                    rgba[idx + 1] = accent_color.1;
                    rgba[idx + 2] = accent_color.2;
                    rgba[idx + 3] = accent_color.3;
                }
            } else {
                rgba[idx] = 0;
                rgba[idx + 1] = 0;
                rgba[idx + 2] = 0;
                rgba[idx + 3] = 0;
            }
        }
    }

    rgba
}

pub fn icon_png() -> Vec<u8> {
    let rgba = icon_rgba();
    let img = image::RgbaImage::from_raw(32, 32, rgba)
        .expect("icon_rgba() always produces exactly 32×32×4 bytes");
    let mut png_bytes = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut png_bytes);
    img.write_to(&mut cursor, image::ImageFormat::Png)
        .expect("writing PNG to an in-memory buffer should never fail");
    png_bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icon_rgba_has_correct_length() {
        let rgba = icon_rgba();
        assert_eq!(rgba.len(), 32 * 32 * 4, "expected 32×32×4 = 4096 bytes");
    }

    #[test]
    fn test_icon_rgba_alpha_values_are_opaque_or_transparent() {
        // Every pixel must be fully opaque (255) or fully transparent (0).
        for (i, chunk) in icon_rgba().chunks_exact(4).enumerate() {
            assert!(
                chunk[3] == 0 || chunk[3] == 255,
                "pixel {i} has unexpected alpha {}",
                chunk[3]
            );
        }
    }

    #[test]
    fn test_icon_png_has_valid_magic_header() {
        let png = icon_png();
        assert!(!png.is_empty(), "PNG output must not be empty");
        // Standard PNG signature
        assert_eq!(
            &png[..8],
            &[137u8, 80, 78, 71, 13, 10, 26, 10],
            "output does not start with the PNG magic header"
        );
    }
}
