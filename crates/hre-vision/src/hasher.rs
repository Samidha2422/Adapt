use crate::validate::{validate_image, VisionError};

/// Calculates the Hamming distance between two 64-bit hashes.
pub fn hamming_distance(left: u64, right: u64) -> u32 {
    (left ^ right).count_ones()
}

/// Computes a 64-bit difference hash from an 8-row by 9-column grayscale intensity grid.
///
/// In dHash, each pixel is compared to its horizontal neighbor (left > right).
/// 8 rows * 8 comparisons = 64 boolean bits.
pub fn compute_dhash_from_grid(grid: &[[u8; 9]; 8]) -> u64 {
    let mut hash = 0u64;
    for (y, row) in grid.iter().enumerate() {
        for x in 0..8 {
            if row[x] > row[x + 1] {
                hash |= 1u64 << (y * 8 + x);
            }
        }
    }
    hash
}

/// Derives a deterministic encoded-byte proxy, NOT a perceptual image hash.
///
/// No pixels are decoded. Kept under the existing name for caller compatibility;
/// unsuitable for perceptual matching or NCII identification.
pub fn compute_phash(bytes: &[u8], declared_mime: &str) -> Result<u64, VisionError> {
    let _meta = validate_image(bytes, declared_mime)?;

    // Use payload bytes after header (skip magic / initial chunk header)
    let payload = if bytes.len() > 32 {
        &bytes[16..]
    } else {
        bytes
    };

    let mut grid = [[0u8; 9]; 8];
    let num_bins = 72;
    let bin_size = (payload.len() / num_bins).max(1);

    for (i, item) in grid.iter_mut().flat_map(|row| row.iter_mut()).enumerate() {
        let start = (i * bin_size).min(payload.len().saturating_sub(1));
        let end = ((i + 1) * bin_size).min(payload.len()).max(start + 1);
        let slice = &payload[start..end];
        let sum: usize = slice.iter().map(|&b| b as usize).sum();
        *item = (sum / slice.len().max(1)) as u8;
    }

    Ok(compute_dhash_from_grid(&grid))
}
