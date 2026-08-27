use std::f64::consts::{FRAC_PI_2, TAU};

use tauri::image::Image;

use crate::config::Theme;

const SIZE: u32 = 32;
const SUPERSAMPLE: u32 = 3;

pub fn render_usage_icon(used_percent: Option<f64>, theme: Theme, stale: bool) -> Image<'static> {
    let palette = Palette::for_theme(theme, stale);
    let normalized = used_percent
        .filter(|value| value.is_finite())
        .map(|value| value.clamp(0.0, 100.0));
    let mut pixels = vec![0_u8; (SIZE * SIZE * 4) as usize];

    for y in 0..SIZE {
        for x in 0..SIZE {
            let mut track_coverage = 0_u32;
            let mut progress_coverage = 0_u32;
            for sample_y in 0..SUPERSAMPLE {
                for sample_x in 0..SUPERSAMPLE {
                    let px = f64::from(x) + (f64::from(sample_x) + 0.5) / f64::from(SUPERSAMPLE);
                    let py = f64::from(y) + (f64::from(sample_y) + 0.5) / f64::from(SUPERSAMPLE);
                    let dx = px - f64::from(SIZE) / 2.0;
                    let dy = py - f64::from(SIZE) / 2.0;
                    let distance = (dx * dx + dy * dy).sqrt();
                    if (distance - 13.0).abs() <= 1.7 {
                        track_coverage += 1;
                        if let Some(percent) = normalized {
                            let angle = (dy.atan2(dx) + FRAC_PI_2).rem_euclid(TAU);
                            if angle <= percent / 100.0 * TAU {
                                progress_coverage += 1;
                            }
                        }
                    }
                }
            }
            let samples = SUPERSAMPLE * SUPERSAMPLE;
            if track_coverage > 0 {
                let base = if progress_coverage > 0 {
                    palette.progress
                } else {
                    palette.track
                };
                let coverage = if progress_coverage > 0 {
                    progress_coverage
                } else {
                    track_coverage
                };
                put_pixel(&mut pixels, x, y, base, coverage * 255 / samples);
            }
        }
    }

    let label = normalized
        .map(|percent| format!("{}", percent.round() as u8))
        .unwrap_or_else(|| "-".to_owned());
    draw_label(&mut pixels, &label, palette.text);
    Image::new_owned(pixels, SIZE, SIZE)
}

#[derive(Clone, Copy)]
struct Palette {
    track: [u8; 3],
    progress: [u8; 3],
    text: [u8; 3],
}

impl Palette {
    fn for_theme(theme: Theme, stale: bool) -> Self {
        match (theme, stale) {
            (Theme::Dark, false) => Self {
                track: [82, 76, 68],
                progress: [222, 159, 93],
                text: [248, 242, 230],
            },
            (Theme::Dark, true) => Self {
                track: [72, 69, 65],
                progress: [150, 112, 76],
                text: [190, 184, 173],
            },
            (Theme::Light, false) => Self {
                track: [190, 183, 173],
                progress: [165, 95, 43],
                text: [38, 35, 30],
            },
            (Theme::Light, true) => Self {
                track: [202, 197, 188],
                progress: [180, 134, 98],
                text: [104, 98, 89],
            },
        }
    }
}

fn draw_label(pixels: &mut [u8], label: &str, color: [u8; 3]) {
    let scale = if label.len() <= 2 { 2 } else { 1 };
    let glyph_width = 3 * scale;
    let gap = scale;
    let total_width = label.len() as u32 * glyph_width + label.len().saturating_sub(1) as u32 * gap;
    let start_x = (SIZE.saturating_sub(total_width)) / 2;
    let start_y = (SIZE - 5 * scale) / 2;

    for (index, character) in label.chars().enumerate() {
        let glyph = glyph(character);
        let origin_x = start_x + index as u32 * (glyph_width + gap);
        for row in 0..5_u32 {
            for column in 0..3_u32 {
                if glyph[row as usize] & (1 << (2 - column)) == 0 {
                    continue;
                }
                for offset_y in 0..scale {
                    for offset_x in 0..scale {
                        put_pixel(
                            pixels,
                            origin_x + column * scale + offset_x,
                            start_y + row * scale + offset_y,
                            color,
                            255,
                        );
                    }
                }
            }
        }
    }
}

fn glyph(character: char) -> [u8; 5] {
    match character {
        '0' => [0b111, 0b101, 0b101, 0b101, 0b111],
        '1' => [0b010, 0b110, 0b010, 0b010, 0b111],
        '2' => [0b111, 0b001, 0b111, 0b100, 0b111],
        '3' => [0b111, 0b001, 0b111, 0b001, 0b111],
        '4' => [0b101, 0b101, 0b111, 0b001, 0b001],
        '5' => [0b111, 0b100, 0b111, 0b001, 0b111],
        '6' => [0b111, 0b100, 0b111, 0b101, 0b111],
        '7' => [0b111, 0b001, 0b010, 0b010, 0b010],
        '8' => [0b111, 0b101, 0b111, 0b101, 0b111],
        '9' => [0b111, 0b101, 0b111, 0b001, 0b111],
        _ => [0b000, 0b000, 0b111, 0b000, 0b000],
    }
}

fn put_pixel(pixels: &mut [u8], x: u32, y: u32, color: [u8; 3], alpha: u32) {
    if x >= SIZE || y >= SIZE {
        return;
    }
    let index = ((y * SIZE + x) * 4) as usize;
    pixels[index] = color[0];
    pixels[index + 1] = color[1];
    pixels[index + 2] = color[2];
    pixels[index + 3] = alpha.min(255) as u8;
}

#[cfg(test)]
mod tests {
    use super::{render_usage_icon, SIZE};
    use crate::config::Theme;

    #[test]
    fn renders_expected_rgba_dimensions() {
        let image = render_usage_icon(Some(24.0), Theme::Dark, false);
        assert_eq!(image.width(), SIZE);
        assert_eq!(image.height(), SIZE);
        assert_eq!(image.rgba().len(), (SIZE * SIZE * 4) as usize);
        assert!(image.rgba().chunks_exact(4).any(|pixel| pixel[3] > 0));
    }

    #[test]
    fn handles_boundary_and_unavailable_values() {
        for value in [Some(-1.0), Some(0.0), Some(100.0), Some(101.0), None] {
            let image = render_usage_icon(value, Theme::Light, true);
            assert_eq!(image.rgba().len(), (SIZE * SIZE * 4) as usize);
        }
    }
}
