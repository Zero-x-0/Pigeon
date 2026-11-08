use eframe::egui;

pub fn get_app_icon() -> egui::IconData {
    let width = 32;
    let height = 32;
    let mut rgba = vec![0u8; (width * height * 4) as usize];

    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;

            // USB Metal Plug on top (x from 10 to 21, y from 3 to 10)
            if x >= 10 && x <= 21 && y >= 3 && y <= 10 {
                // Outer plug boundary
                if x == 10 || x == 21 || y == 3 {
                    rgba[idx] = 160;     // R
                    rgba[idx + 1] = 165; // G
                    rgba[idx + 2] = 175; // B
                    rgba[idx + 3] = 255; // A
                } else if (x == 13 || x == 14 || x == 17 || x == 18) && (y >= 5 && y <= 7) {
                    // USB Connector Holes
                    rgba[idx] = 60;
                    rgba[idx + 1] = 60;
                    rgba[idx + 2] = 60;
                    rgba[idx + 3] = 255;
                } else {
                    // Plug metallic body
                    rgba[idx] = 210;
                    rgba[idx + 1] = 215;
                    rgba[idx + 2] = 225;
                    rgba[idx + 3] = 255;
                }
                continue;
            }

            // USB Drive Plastic Body (x from 8 to 23, y from 11 to 29)
            if x >= 8 && x <= 23 && y >= 11 && y <= 29 {
                // Rounded corner cutouts
                if (x == 8 && (y == 11 || y == 29))
                    || (x == 23 && (y == 11 || y == 29))
                {
                    continue;
                }

                // LED activity light (centered at y=26, x=15..16)
                if (x == 15 || x == 16) && y == 26 {
                    rgba[idx] = 0;       // Green LED
                    rgba[idx + 1] = 230;
                    rgba[idx + 2] = 118;
                    rgba[idx + 3] = 255;
                    continue;
                }

                // White "X" emblem in center (y from 16 to 22)
                let rel_y = y as i32 - 19;
                let rel_x = x as i32 - 15;
                if (rel_x.abs() == rel_y.abs() && rel_x.abs() <= 3) || (rel_x == 0 && rel_y == 0) {
                    rgba[idx] = 255;
                    rgba[idx + 1] = 255;
                    rgba[idx + 2] = 255;
                    rgba[idx + 3] = 255;
                    continue;
                }

                // Primary Drive Body - Vibrant Rufus / Windows Blue
                if x == 8 || x == 23 || y == 11 || y == 29 {
                    // Border shadow
                    rgba[idx] = 0;
                    rgba[idx + 1] = 90;
                    rgba[idx + 2] = 180;
                    rgba[idx + 3] = 255;
                } else {
                    // Gradient blue body
                    let factor = (y - 11) as f32 / 18.0;
                    rgba[idx] = (0.0 * (1.0 - factor) + 15.0 * factor) as u8;
                    rgba[idx + 1] = (120.0 * (1.0 - factor) + 100.0 * factor) as u8;
                    rgba[idx + 2] = (215.0 * (1.0 - factor) + 200.0 * factor) as u8;
                    rgba[idx + 3] = 255;
                }
            }
        }
    }

    egui::IconData {
        rgba,
        width,
        height,
    }
}

