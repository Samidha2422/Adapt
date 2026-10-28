use serde::{Deserialize, Serialize};

pub const MAX_IMAGE_SIZE: usize = 10 * 1024 * 1024; // 10 MiB
pub const MAX_DIMENSION: u32 = 4096;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageMetadata {
    pub mime: String,
    pub width: u32,
    pub height: u32,
    pub size_bytes: usize,
}

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum VisionError {
    #[error("empty image input")]
    EmptyInput,
    #[error("image size {size} bytes exceeds maximum allowed {max} bytes")]
    Oversized { size: usize, max: usize },
    #[error("image truncated or malformed header")]
    TruncatedImage,
    #[error("unsupported image format")]
    UnsupportedFormat,
    #[error("declared MIME '{declared}' does not match detected format '{detected}'")]
    MimeMismatch { declared: String, detected: String },
    #[error("image dimensions {width}x{height} exceed limit of {max}x{max}")]
    OversizedDimensions { width: u32, height: u32, max: u32 },
    #[error("invalid image dimensions")]
    InvalidDimensions,
    #[error("OCR error: {0}")]
    Ocr(String),
}

/// Sniffs magic bytes and extracts image dimensions and MIME type.
pub fn validate_image(bytes: &[u8], declared_mime: &str) -> Result<ImageMetadata, VisionError> {
    if bytes.is_empty() {
        return Err(VisionError::EmptyInput);
    }
    if bytes.len() > MAX_IMAGE_SIZE {
        return Err(VisionError::Oversized {
            size: bytes.len(),
            max: MAX_IMAGE_SIZE,
        });
    }

    let detected_mime = sniff_mime(bytes)?;
    let normalized_declared = declared_mime.trim().to_lowercase();
    if normalized_declared != detected_mime
        && !(normalized_declared == "image/jpg" && detected_mime == "image/jpeg")
    {
        return Err(VisionError::MimeMismatch {
            declared: declared_mime.to_string(),
            detected: detected_mime.to_string(),
        });
    }

    let (width, height) = parse_dimensions(bytes, detected_mime)?;
    if width == 0 || height == 0 {
        return Err(VisionError::InvalidDimensions);
    }
    if width > MAX_DIMENSION || height > MAX_DIMENSION {
        return Err(VisionError::OversizedDimensions {
            width,
            height,
            max: MAX_DIMENSION,
        });
    }

    Ok(ImageMetadata {
        mime: detected_mime.to_string(),
        width,
        height,
        size_bytes: bytes.len(),
    })
}

fn sniff_mime(bytes: &[u8]) -> Result<&'static str, VisionError> {
    if bytes.len() < 8 {
        return Err(VisionError::TruncatedImage);
    }
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        return Ok("image/png");
    }
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Ok("image/jpeg");
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Ok("image/gif");
    }
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Ok("image/webp");
    }
    Err(VisionError::UnsupportedFormat)
}

fn parse_dimensions(bytes: &[u8], mime: &str) -> Result<(u32, u32), VisionError> {
    match mime {
        "image/png" => parse_png_dimensions(bytes),
        "image/gif" => parse_gif_dimensions(bytes),
        "image/jpeg" => parse_jpeg_dimensions(bytes),
        "image/webp" => parse_webp_dimensions(bytes),
        _ => Err(VisionError::UnsupportedFormat),
    }
}

fn parse_png_dimensions(bytes: &[u8]) -> Result<(u32, u32), VisionError> {
    if bytes.len() < 24 {
        return Err(VisionError::TruncatedImage);
    }
    if &bytes[12..16] != b"IHDR" {
        return Err(VisionError::TruncatedImage);
    }
    let mut w_bytes = [0u8; 4];
    w_bytes.copy_from_slice(&bytes[16..20]);
    let width = u32::from_be_bytes(w_bytes);

    let mut h_bytes = [0u8; 4];
    h_bytes.copy_from_slice(&bytes[20..24]);
    let height = u32::from_be_bytes(h_bytes);

    Ok((width, height))
}

fn parse_gif_dimensions(bytes: &[u8]) -> Result<(u32, u32), VisionError> {
    if bytes.len() < 10 {
        return Err(VisionError::TruncatedImage);
    }
    let mut w_bytes = [0u8; 2];
    w_bytes.copy_from_slice(&bytes[6..8]);
    let width = u16::from_le_bytes(w_bytes) as u32;

    let mut h_bytes = [0u8; 2];
    h_bytes.copy_from_slice(&bytes[8..10]);
    let height = u16::from_le_bytes(h_bytes) as u32;

    Ok((width, height))
}

fn parse_jpeg_dimensions(bytes: &[u8]) -> Result<(u32, u32), VisionError> {
    let mut pos = 2; // skip 0xFF, 0xD8
    while pos + 4 <= bytes.len() {
        if bytes[pos] != 0xFF {
            pos += 1;
            continue;
        }
        // Skip fill bytes
        while pos < bytes.len() && bytes[pos] == 0xFF {
            pos += 1;
        }
        if pos >= bytes.len() {
            break;
        }
        let marker = bytes[pos];
        pos += 1;

        // EOI or SOS
        if marker == 0xD9 || marker == 0xDA {
            break;
        }

        if pos + 2 > bytes.len() {
            return Err(VisionError::TruncatedImage);
        }
        let len = u16::from_be_bytes([bytes[pos], bytes[pos + 1]]) as usize;
        if len < 2 {
            return Err(VisionError::TruncatedImage);
        }

        // SOF0 (0xC0) to SOF3 (0xC3), SOF5..SOF7, SOF9..SOF11, SOF13..SOF15
        if matches!(
            marker,
            0xC0..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF
        ) {
            if pos + 7 > bytes.len() {
                return Err(VisionError::TruncatedImage);
            }
            let height = u16::from_be_bytes([bytes[pos + 3], bytes[pos + 4]]) as u32;
            let width = u16::from_be_bytes([bytes[pos + 5], bytes[pos + 6]]) as u32;
            return Ok((width, height));
        }

        pos += len;
    }
    Err(VisionError::TruncatedImage)
}

fn parse_webp_dimensions(bytes: &[u8]) -> Result<(u32, u32), VisionError> {
    if bytes.len() < 30 {
        return Err(VisionError::TruncatedImage);
    }
    let chunk_type = &bytes[12..16];
    if chunk_type == b"VP8 " {
        if bytes.len() < 30 {
            return Err(VisionError::TruncatedImage);
        }
        let w = u16::from_le_bytes([bytes[26], bytes[27]]) as u32 & 0x3FFF;
        let h = u16::from_le_bytes([bytes[28], bytes[29]]) as u32 & 0x3FFF;
        Ok((w, h))
    } else if chunk_type == b"VP8L" {
        if bytes.len() < 25 {
            return Err(VisionError::TruncatedImage);
        }
        let b0 = bytes[21] as u32;
        let b1 = bytes[22] as u32;
        let b2 = bytes[23] as u32;
        let b3 = bytes[24] as u32;
        let w = 1 + (b0 | ((b1 & 0x3F) << 8));
        let h = 1 + ((b1 >> 6) | (b2 << 2) | ((b3 & 0x0F) << 10));
        Ok((w, h))
    } else if chunk_type == b"VP8X" {
        if bytes.len() < 30 {
            return Err(VisionError::TruncatedImage);
        }
        let w = 1 + (bytes[24] as u32 | ((bytes[25] as u32) << 8) | ((bytes[26] as u32) << 16));
        let h = 1 + (bytes[27] as u32 | ((bytes[28] as u32) << 8) | ((bytes[29] as u32) << 16));
        Ok((w, h))
    } else {
        Err(VisionError::UnsupportedFormat)
    }
}
