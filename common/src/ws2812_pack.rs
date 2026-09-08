//! Pixel packing for the WS2812 / SK6812 PIO driver (`pico2/src/ws2812.rs`).
//!
//! The state machine shifts a continuous MSB-first bit stream out of 32-bit
//! words; pixel framing is entirely in the data. These two functions build that
//! stream — pure arithmetic, so they live here where `host_tests` can check the
//! word-boundary handling that would otherwise be verified only by looking at
//! LEDs.

use smart_leds::RGBW;

/// Words needed for `n` 24-bit pixels: `ceil(n * 24 / 32)`.
pub const fn grb_words(n: usize) -> usize {
    (n * 24).div_ceil(32)
}

/// Pack GRB pixels (24 bits each) into a continuous MSB-first bit stream.
/// Returns the number of words written; `out` must hold [`grb_words`].
pub fn pack_grb(pixels: &[RGBW<u8>], out: &mut [u32]) -> usize {
    let mut acc: u64 = 0;
    let mut bits: u32 = 0;
    let mut n = 0usize;
    for p in pixels {
        let v = ((p.g as u64) << 16) | ((p.r as u64) << 8) | (p.b as u64);
        acc = (acc << 24) | v;
        bits += 24;
        if bits >= 32 {
            let shift = bits - 32;
            out[n] = (acc >> shift) as u32;
            n += 1;
            bits -= 32;
            acc &= (1u64 << bits) - 1;
        }
    }
    if bits > 0 {
        // Left-justify the tail; the surplus low bits clock out after the last
        // pixel and are forwarded by the last LED to nothing.
        out[n] = (acc << (32 - bits)) as u32;
        n += 1;
    }
    n
}

/// Pack GRBW pixels, one 32-bit word each. Returns the number of words written.
pub fn pack_grbw(pixels: &[RGBW<u8>], out: &mut [u32]) -> usize {
    for (w, p) in out.iter_mut().zip(pixels) {
        *w = ((p.g as u32) << 24) | ((p.r as u32) << 16) | ((p.b as u32) << 8) | (p.a.0 as u32);
    }
    pixels.len().min(out.len())
}
