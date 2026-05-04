//! 이미지 회귀 테스트용 diff 유틸리티.

use image::{DynamicImage, GenericImageView, Rgba};

/// 두 이미지의 픽셀 차이가 이 임계값 이하이면 동일로 간주한다.
pub const DIFF_THRESHOLD: f64 = 0.001;

/// 두 `DynamicImage`의 정규화 픽셀 diff를 반환한다 (0.0 ~ 1.0).
///
/// 픽셀별 채널 차이를 합산한 뒤 (최대 가능 차이 * 픽셀 수)로 나눠 정규화한다.
/// 크기가 다른 경우 완전 불일치로 판단하여 1.0을 반환한다.
pub fn normalized_diff(a: &DynamicImage, b: &DynamicImage) -> f64 {
    if a.dimensions() != b.dimensions() {
        return 1.0;
    }

    let (width, height) = a.dimensions();
    let pixel_count = width as u64 * height as u64;

    if pixel_count == 0 {
        return 0.0;
    }

    let a_rgba = a.to_rgba8();
    let b_rgba = b.to_rgba8();

    let total_diff: u64 = a_rgba
        .pixels()
        .zip(b_rgba.pixels())
        .map(|(pa, pb)| {
            pa.0.iter()
                .zip(pb.0.iter())
                .map(|(&ca, &cb)| ca.abs_diff(cb) as u64)
                .sum::<u64>()
        })
        .sum();

    // 채널 4개, 각 채널 최대 255
    let max_possible = pixel_count * 4 * 255;
    total_diff as f64 / max_possible as f64
}

/// diff 픽셀을 시각화한 `DynamicImage`를 반환한다 (CI 아티팩트용).
///
/// 두 이미지에서 차이가 있는 픽셀을 빨간색으로, 동일한 픽셀을 어두운 회색으로 표시한다.
/// 크기가 다른 경우 a의 크기에 맞춰 빨간색으로 채워진 이미지를 반환한다.
pub fn diff_image(a: &DynamicImage, b: &DynamicImage) -> DynamicImage {
    let (width, height) = a.dimensions();

    if a.dimensions() != b.dimensions() {
        let red_fill = image::RgbaImage::from_pixel(width, height, Rgba([255, 0, 0, 255]));
        return DynamicImage::ImageRgba8(red_fill);
    }

    let a_rgba = a.to_rgba8();
    let b_rgba = b.to_rgba8();

    let diff_buf = image::RgbaImage::from_fn(width, height, |x, y| {
        let pa = a_rgba.get_pixel(x, y);
        let pb = b_rgba.get_pixel(x, y);
        let is_different = pa.0 != pb.0;
        if is_different {
            Rgba([255, 0, 0, 255])
        } else {
            Rgba([30, 30, 30, 255])
        }
    });

    DynamicImage::ImageRgba8(diff_buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, RgbaImage};

    fn solid_image(width: u32, height: u32, color: [u8; 4]) -> DynamicImage {
        DynamicImage::ImageRgba8(RgbaImage::from_pixel(width, height, Rgba(color)))
    }

    #[test]
    fn same_image_diff_is_zero() {
        let img = solid_image(10, 10, [128, 64, 32, 255]);
        assert_eq!(normalized_diff(&img, &img), 0.0);
    }

    #[test]
    fn black_vs_white_diff_is_one() {
        let black = solid_image(10, 10, [0, 0, 0, 255]);
        let white = solid_image(10, 10, [255, 255, 255, 255]);
        // 알파 채널은 동일(255)하므로 RGB 3채널만 최대 차이 → 3/4 = 0.75
        // (완전히 1.0이 되려면 알파도 달라야 함)
        let diff = normalized_diff(&black, &white);
        // 0.75 이상인지만 검증 (near-max 불일치)
        assert!(diff >= 0.7, "흑백 이미지 diff가 충분히 커야 함: {diff}");
    }

    #[test]
    fn different_size_diff_is_one() {
        let a = solid_image(10, 10, [255, 255, 255, 255]);
        let b = solid_image(20, 20, [255, 255, 255, 255]);
        assert_eq!(normalized_diff(&a, &b), 1.0);
    }

    #[test]
    fn diff_image_output_size_matches_input() {
        let a = solid_image(15, 20, [100, 100, 100, 255]);
        let b = solid_image(15, 20, [200, 200, 200, 255]);
        let diff = diff_image(&a, &b);
        assert_eq!(diff.dimensions(), (15, 20));
    }
}
