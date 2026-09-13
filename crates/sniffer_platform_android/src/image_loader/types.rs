//! Asynchronous image loading request and result data structures.

/// Request payload sent to the asynchronous image loading worker.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ImageLoadRequest {
    /// Unique identifier for this image resource (often same as `src` or specified by caller).
    pub id: String,
    /// Path or URI of the image (e.g. `.plugins/my_plugin/icon.png`).
    pub src: String,
}

impl ImageLoadRequest {
    /// Creates a new `ImageLoadRequest` with explicit `id` and `src`.
    #[must_use]
    pub fn new(id: impl Into<String>, src: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            src: src.into(),
        }
    }

    /// Creates a new `ImageLoadRequest` where `id` is identical to `src`.
    #[must_use]
    pub fn from_src(src: impl Into<String>) -> Self {
        let s = src.into();
        Self {
            id: s.clone(),
            src: s,
        }
    }
}

/// Result payload containing decoded RGBA8 pixel data and metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImageLoadResult {
    /// Unique identifier matching the request.
    pub id: String,
    /// Path or URI matching the original request.
    pub src: String,
    /// Raw RGBA8 pixel buffer (length should equal `width * height * 4`).
    pub pixels: Vec<u8>,
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
}

impl ImageLoadResult {
    /// Creates a new `ImageLoadResult`.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        src: impl Into<String>,
        pixels: Vec<u8>,
        width: u32,
        height: u32,
    ) -> Self {
        Self {
            id: id.into(),
            src: src.into(),
            pixels,
            width,
            height,
        }
    }

    /// Returns `true` if the pixel buffer length matches `width * height * 4` and dimensions are non-zero.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        if self.width == 0 || self.height == 0 {
            return false;
        }
        let expected_len = (self.width as usize)
            .saturating_mul(self.height as usize)
            .saturating_mul(4);
        self.pixels.len() == expected_len
    }

    /// Returns the total number of bytes in the pixel buffer.
    #[must_use]
    pub fn byte_len(&self) -> usize {
        self.pixels.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_load_request_construction() {
        let req1 = ImageLoadRequest::new("img1", "path/to/icon.png");
        assert_eq!(req1.id, "img1");
        assert_eq!(req1.src, "path/to/icon.png");

        let req2 = ImageLoadRequest::from_src("assets/logo.png");
        assert_eq!(req2.id, "assets/logo.png");
        assert_eq!(req2.src, "assets/logo.png");
    }

    #[test]
    fn test_image_load_result_validation() {
        let width = 2;
        let height = 2;
        let pixels = vec![255; (width * height * 4) as usize];
        let result = ImageLoadResult::new("img1", "path/to/icon.png", pixels, width, height);

        assert!(result.is_valid());
        assert_eq!(result.byte_len(), 16);
        assert_eq!(result.id, "img1");
        assert_eq!(result.src, "path/to/icon.png");
        assert_eq!(result.width, 2);
        assert_eq!(result.height, 2);
    }

    #[test]
    fn test_image_load_result_invalid_dimensions() {
        let invalid_res1 = ImageLoadResult::new("id", "src", vec![0; 16], 0, 4);
        assert!(!invalid_res1.is_valid());

        let invalid_res2 = ImageLoadResult::new("id", "src", vec![0; 15], 2, 2);
        assert!(!invalid_res2.is_valid());
    }
}
