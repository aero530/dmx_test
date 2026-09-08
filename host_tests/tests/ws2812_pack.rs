//! The WS2812 bit-packer feeds a PIO state machine that shifts whole words
//! MSB-first; a wrong shift here corrupts every fourth pixel on the strip.

use common::ws2812_pack::{grb_words, pack_grb, pack_grbw};
use smart_leds::{White, RGBW};

fn px(r: u8, g: u8, b: u8, w: u8) -> RGBW<u8> {
    RGBW { r, g, b, a: White(w) }
}

#[test]
fn grb_pixels_pack_across_word_boundaries() {
    // GRB values: 0x221133 0x554466 0x887799 0xBBAACC — 96 bits = 3 words.
    let pixels = [px(0x11, 0x22, 0x33, 0), px(0x44, 0x55, 0x66, 0), px(0x77, 0x88, 0x99, 0), px(0xAA, 0xBB, 0xCC, 0)];
    let mut out = [0u32; 4];
    let n = pack_grb(&pixels, &mut out);
    assert_eq!(n, 3);
    assert_eq!(&out[..3], &[0x2211_3355, 0x4466_8877, 0x99BB_AACC]);
}

#[test]
fn grb_tail_is_left_justified() {
    let mut out = [0u32; 2];
    // One pixel: 24 bits in the top of one word, padding below.
    assert_eq!(pack_grb(&[px(0x11, 0x22, 0x33, 0xFF)], &mut out), 1);
    assert_eq!(out[0], 0x2211_3300, "white byte must not leak into RGB output");
    // Two pixels: 48 bits -> two words, the second half-full.
    assert_eq!(pack_grb(&[px(0x11, 0x22, 0x33, 0), px(0x44, 0x55, 0x66, 0)], &mut out), 2);
    assert_eq!(out, [0x2211_3355, 0x4466_0000]);
    // Nothing to send is nothing.
    assert_eq!(pack_grb(&[], &mut out), 0);
}

#[test]
fn grb_word_count_matches_the_packer() {
    for n in 0..=600 {
        let pixels = vec![px(1, 2, 3, 4); n];
        let mut out = vec![0u32; grb_words(n)];
        assert_eq!(pack_grb(&pixels, &mut out), grb_words(n), "n = {n}");
    }
    assert_eq!(grb_words(600), 450);
}

#[test]
fn grbw_is_one_word_per_pixel_in_wire_order() {
    let mut out = [0u32; 2];
    assert_eq!(pack_grbw(&[px(0x11, 0x22, 0x33, 0x44)], &mut out), 1);
    assert_eq!(out[0], 0x2211_3344);
    // Never writes past the buffer it was given.
    assert_eq!(pack_grbw(&[px(1, 1, 1, 1); 3], &mut out), 2);
}
