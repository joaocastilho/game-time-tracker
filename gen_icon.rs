pub mod icon {
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
}

fn main() {
    let rgba = icon::icon_rgba();
    
    let mut out = std::fs::File::create("assets/icon.ico").unwrap();
    use std::io::Write;
    
    // ICONDIR
    out.write_all(&[0, 0, 1, 0, 1, 0]).unwrap(); // Reserved, Type (1=ico), Count (1)
    
    // ICONDIRENTRY
    out.write_all(&[
        32, 32, // Width, Height
        0, 0,   // Color count, Reserved
        1, 0,   // Color planes
        32, 0,  // Bits per pixel
    ]).unwrap();
    
    let image_size: u32 = 40 + (32 * 32 * 4) + (32 * 32 / 8); // BITMAPINFOHEADER + pixels + AND mask
    out.write_all(&image_size.to_le_bytes()).unwrap(); // Size in bytes
    out.write_all(&22u32.to_le_bytes()).unwrap(); // Offset
    
    // BITMAPINFOHEADER
    out.write_all(&40u32.to_le_bytes()).unwrap(); // Size of header
    out.write_all(&32i32.to_le_bytes()).unwrap(); // Width
    out.write_all(&(32i32 * 2).to_le_bytes()).unwrap(); // Height (multiplied by 2 for mask)
    out.write_all(&1u16.to_le_bytes()).unwrap();  // Color planes
    out.write_all(&32u16.to_le_bytes()).unwrap(); // Bits per pixel
    out.write_all(&0u32.to_le_bytes()).unwrap();  // Compression (0 = BI_RGB)
    out.write_all(&(image_size - 40).to_le_bytes()).unwrap(); // Image size
    out.write_all(&0i32.to_le_bytes()).unwrap();  // X pixels per meter
    out.write_all(&0i32.to_le_bytes()).unwrap();  // Y pixels per meter
    out.write_all(&0u32.to_le_bytes()).unwrap();  // Colors used
    out.write_all(&0u32.to_le_bytes()).unwrap();  // Important colors
    
    // Pixel data (bottom-up, BGRA)
    for y in (0..32).rev() {
        for x in 0..32 {
            let idx = (y * 32 + x) * 4;
            let r = rgba[idx];
            let g = rgba[idx + 1];
            let b = rgba[idx + 2];
            let a = rgba[idx + 3];
            out.write_all(&[b, g, r, a]).unwrap(); // Write BGRA
        }
    }
    
    // AND mask (1 bit per pixel, padding to 32-bit boundary)
    // All 0s meaning "use alpha channel"
    let mask_row = vec![0u8; 4];
    for _ in 0..32 {
        out.write_all(&mask_row).unwrap();
    }
    
    println!("Icon successfully saved to assets/icon.ico");
}
