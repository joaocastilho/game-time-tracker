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
    let img = image::RgbaImage::from_raw(32, 32, rgba).unwrap();
    let mut png_bytes = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut png_bytes);
    img.write_to(&mut cursor, image::ImageFormat::Png).unwrap();
    png_bytes
}
