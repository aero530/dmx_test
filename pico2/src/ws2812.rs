//! WS2812 / SK6812 output — one PIO state machine per string, 24- or 32-bit
//! pixels chosen per frame.
//!
//! embassy-rp ships `PioWs2812` (24-bit) and `RgbwPioWs2812` (32-bit) as two
//! drivers that each own a state machine, so a board that lets the user pick
//! RGB or RGBW at runtime cannot use them directly. This driver runs the same
//! PIO program with a **32-bit autopull threshold** and takes a pre-packed bit
//! stream: the pixel framing lives entirely in the data. Twenty-four-bit GRB
//! pixels are packed back-to-back across word boundaries (450 words per 600
//! LEDs), 32-bit GRBW pixels are one word each. Up to 31 surplus bits at the
//! end of the last word clock out after the final pixel; the last LED forwards
//! them to nothing, so they are harmless.
//!
//! Timing is the canonical 10-cycle WS2812 program (T1 = 2, T2 = 5, T3 = 3 at
//! 8 MHz → 1.25 µs/bit), which SK6812 RGBW parts share. After each frame the
//! FIFO is drained and the line held idle 150 µs: WS2812 needs > 50 µs of
//! latch, SK6812 > 80 µs, and the OSR can still hold one word (40 µs).

use embassy_rp::clocks::clk_sys_freq;
use embassy_rp::dma;
use embassy_rp::interrupt;
use embassy_rp::pio::program::pio_asm;
use embassy_rp::pio::{
    Common, Config, Direction, FifoJoin, Instance, LoadedProgram, PioPin, ShiftConfig,
    ShiftDirection, StateMachine,
};
use embassy_rp::Peri;
use embassy_time::Timer;
use fixed::types::U24F8;

/// Bit rate on the wire.
const BIT_RATE_HZ: u32 = 800_000;
/// PIO cycles per bit in the program below (T1 + T2 + T3).
const CYCLES_PER_BIT: u32 = 10;

/// The WS2812 program, loaded once per PIO block and shared by its state
/// machines.
pub struct Ws2812Program<'a, PIO: Instance> {
    prg: LoadedProgram<'a, PIO>,
}

impl<'a, PIO: Instance> Ws2812Program<'a, PIO> {
    pub fn new(common: &mut Common<'a, PIO>) -> Self {
        // T1 = 2 cycles high for every bit, T2 = 5 cycles that stay high for a
        // 1 and drop for a 0, T3 = 3 cycles low for every bit.
        let prg = pio_asm!(
            ".side_set 1",
            ".wrap_target",
            "bitloop:",
            "    out x, 1        side 0 [2]", // T3: low tail of the previous bit, fetch next
            "    jmp !x do_zero  side 1 [1]", // T1: rising edge for every bit
            "    jmp bitloop     side 1 [4]", // T2: a 1 stays high
            "do_zero:",
            "    nop             side 0 [4]", // T2: a 0 drops low
            ".wrap",
        );
        let prg = common.load_program(&prg.program);
        Self { prg }
    }
}

/// One output string.
pub struct Ws2812<'d, P: Instance, const S: usize> {
    dma: dma::Channel<'d>,
    sm: StateMachine<'d, P, S>,
}

impl<'d, P: Instance, const S: usize> Ws2812<'d, P, S> {
    pub fn new<D: dma::ChannelInstance>(
        pio: &mut Common<'d, P>,
        mut sm: StateMachine<'d, P, S>,
        dma: Peri<'d, D>,
        irq: impl interrupt::typelevel::Binding<D::Interrupt, dma::InterruptHandler<D>> + 'd,
        pin: Peri<'d, impl PioPin>,
        program: &Ws2812Program<'d, P>,
    ) -> Self {
        let mut cfg = Config::default();

        let out_pin = pio.make_pio_pin(pin);
        cfg.use_program(&program.prg, &[&out_pin]);
        sm.set_pins(embassy_rp::gpio::Level::Low, &[&out_pin]);
        sm.set_pin_dirs(Direction::Out, &[&out_pin]);

        // Clock: measured in kHz to keep the fixed-point maths in range.
        let clock_khz = U24F8::from_num(clk_sys_freq() / 1000);
        let bit_khz = U24F8::from_num(BIT_RATE_HZ / 1000 * CYCLES_PER_BIT);
        cfg.clock_divider = clock_khz / bit_khz;

        cfg.fifo_join = FifoJoin::TxOnly;
        cfg.shift_out = ShiftConfig {
            auto_fill: true,
            threshold: 32, // whole words; pixel framing is in the data
            direction: ShiftDirection::Left, // MSB first
        };

        sm.set_config(&cfg);
        sm.set_enable(true);

        Self {
            dma: dma::Channel::new(dma, irq),
            sm,
        }
    }

    /// Clock a packed bit stream out and hold the latch gap, so back-to-back
    /// calls produce cleanly latched frames.
    ///
    /// DMA completion only means the last word reached the **FIFO** — up to
    /// eight words (256 bits, 320 µs at 800 kHz) can still be queued, and the
    /// OSR holds one more. Timing the gap from there would start the next
    /// frame while this one is still on the wire whenever frames arrive
    /// faster than they render (a USB host pushing label-6 packets at full
    /// rate, say), and WS2812s then run the two frames together. So wait for
    /// the FIFO to drain first, then allow the OSR's 40 µs plus the latch:
    /// WS2812 needs > 50 µs, SK6812 > 80 µs.
    pub async fn write(&mut self, words: &[u32]) {
        if !words.is_empty() {
            self.sm.tx().dma_push(&mut self.dma, words, false).await;
            while !self.sm.tx().empty() {
                Timer::after_micros(40).await;
            }
        }
        Timer::after_micros(150).await;
    }
}

/// Pixel packing lives in `common::ws2812_pack` so the host tests cover it.
pub use common::ws2812_pack::{pack_grb, pack_grbw};
