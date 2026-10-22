//! StatusNotifier pixmaps use network-order ARGB, independent of host endian.
use crate::notification::Image;
pub fn pixmap(images: &[(i32, i32, Vec<u8>)], target: i32) -> Option<Image> {
    let target = target.clamp(1, 512);
    let selected = images
        .iter()
        .take(16)
        .filter(|(width, height, data)| {
            (1..=512).contains(width)
                && (1..=512).contains(height)
                && data.len() == *width as usize * *height as usize * 4
        })
        .min_by_key(|(width, height, _)| {
            (
                (width - target).abs() + (height - target).abs(),
                *width < target,
            )
        });
    let (width, height, data) = selected?;
    let rgba = data
        .chunks_exact(4)
        .flat_map(|pixel| [pixel[1], pixel[2], pixel[3], pixel[0]])
        .collect();
    Some(Image {
        width: *width,
        height: *height,
        rgba,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn argb_is_converted_and_malformed_frames_are_skipped() {
        let images = vec![
            (1, 1, vec![128, 10, 20, 30]),
            (0, 1, vec![]),
            (i32::MAX, i32::MAX, vec![]),
        ];
        assert_eq!(
            pixmap(&images, 1).map(|i| i.rgba),
            Some(vec![10, 20, 30, 128])
        );
        assert!(pixmap(&[(2, 2, vec![0; 15])], 2).is_none());
    }
}
