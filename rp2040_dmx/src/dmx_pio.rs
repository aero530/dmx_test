//! PIO DMX

use fixed::traits::ToFixed;
use embassy_rp::clocks::clk_sys_freq;
use embassy_rp::gpio::Level;
use embassy_rp::Peri;
use embassy_rp::pio::program::pio_asm;
use embassy_rp::pio::{Common, Config, Direction, FifoJoin, Instance, LoadedProgram, PioPin, ShiftDirection, StateMachine};
// use embassy_rp::dma::{AnyChannel, Channel, Transfer};

use super::DMX_SIZE;

/// This struct represents a DMX Rx program loaded into pio instruction memory.
pub struct PioDmxRxProgram<'d, PIO: Instance> {
    prg: LoadedProgram<'d, PIO>,
}

impl<'d, PIO: Instance> PioDmxRxProgram<'d, PIO> {
    /// Load the DMX rx program into the given pio
    pub fn new(common: &mut Common<'d, PIO>) -> Self {
        // The program assumes a PIO clock frequency of exactly 1MHz
        let prg = pio_asm!(
                // ".program DmxInput",
                ".define dmx_bit 4",                     // As DMX has a baudrate of 250.000kBaud, a single bit is 4us
                "break_reset:",
                "    set x, 29",                         // Setup a counter to count the iterations on break_loop
                "break_loop:",                           // Break loop lasts for 8us. The entire break must be minimum 30*3us = 90us
                "    jmp pin break_reset",               // Go back to start if pin goes high during the break
                "    jmp x-- break_loop   [1]",          // Decrease the counter and go back to break loop if x>0 so that the break is not done
                "    wait 1 pin 0",                      // Stall until line goes high for the Mark-After-Break (MAB) 
                ".wrap_target",
                "    wait 0 pin 0",                      // Stall until start bit is asserted
                "    set x, 7             [dmx_bit]",    // Preload bit counter, then delay until halfway through
                "bitloop:",
                "    in pins, 1",                        // Shift data bit into ISR
                "    jmp x-- bitloop      [dmx_bit-2]",  // Loop 8 times, each loop iteration is 4us
                "    wait 1 pin 0",                      // Wait for pin to go high for stop bits
                "    in NULL, 24",                       // Push 24 more bits into the ISR so that our one byte is at the position where the DMA expects it.  8 data bits + 24 dummy bits = 32bit word of FIFO
                "    push",                              // Should probably do error checking on the stop bits some time in the future....
                ".wrap",                                 // Return to wrap_target
        );

        let prg = common.load_program(&prg.program);
        Self { prg }
    }
}

/// PIO backed DMX reciever
pub struct PioDmxRx<'d, PIO: Instance, const SM: usize> {
    sm: StateMachine<'d, PIO, SM>,
    // dma: Peri<'d, AnyChannel>,
}

impl<'d, PIO: Instance, const SM: usize> PioDmxRx<'d, PIO, SM> {
    /// Configure a pio state machine to use the loaded rx program.
    pub fn new(
        // dma: Peri<'d, impl Channel>,
        common: &mut Common<'d, PIO>,
        mut sm: StateMachine<'d, PIO, SM>,
        rx_pin: Peri<'d, impl PioPin>,
        program: &PioDmxRxProgram<'d, PIO>,
    ) -> Self {
        let mut cfg = Config::default();
        
        // Attempt to load the DMX PIO assembly program into the PIO program memory
        cfg.use_program(&program.prg, &[]);

        // Set this pin's GPIO function (connect PIO to the pad)
        // pio_sm_set_consecutive_pindirs(pio, sm, pin, 1, false);
        // pio_gpio_init(pio, pin);
        // gpio_pull_up(pin);
        let rx_pin = common.make_pio_pin(rx_pin);
        sm.set_pins(Level::High, &[&rx_pin]);
        sm.set_pin_dirs(Direction::In, &[&rx_pin]);

        // sm_config_set_in_pins(&sm_conf, pin); // for WAIT, IN
        // sm_config_set_jmp_pin(&sm_conf, pin); // for JMP
        cfg.set_in_pins(&[&rx_pin]);
        cfg.set_jmp_pin(&rx_pin);
        

        // Setup the side-set pins for the PIO state machine
        // Shift to right, autopush disabled
        // sm_config_set_in_shift(&sm_conf, true, false, 8);
        cfg.shift_in.auto_fill = false;
        cfg.shift_in.direction = ShiftDirection::Right;
        cfg.shift_in.threshold = 8;

        // Deeper FIFO as we're not doing any TX
        // sm_config_set_fifo_join(&sm_conf, PIO_FIFO_JOIN_RX);
        cfg.fifo_join = FifoJoin::RxOnly;

        // Setup the clock divider to run the state machine at exactly 1MHz
        // uint clk_div = clock_get_hz(clk_sys) / DMX_SM_FREQ;
        // sm_config_set_clkdiv(&sm_conf, clk_div);
        cfg.clock_divider = (clk_sys_freq() / 1_000_000 ).to_fixed();

        // Load our configuration, jump to the start of the program and run the State Machine
        // pio_sm_init(pio, sm, prgm_offsets[pio_ind], &sm_conf);
        sm.set_config(&cfg);
        sm.set_enable(true);

        Self { 
            sm, 
            // dma: dma.into()
        }
        
    }

    /// Wait for a single u8
    pub async fn read_u8(&mut self) -> u8 {
        self.sm.rx().wait_pull().await as u8
    }
    
    /// Return an in-prograss dma transfer future. Awaiting it will guarentee a complete transfer.
    pub async fn read<'b>(&'b mut self, buff: &'b mut [u8; DMX_SIZE]) {
        // wait for start byte
        // push data to buffer [512]
        
        let mut i = 0;

        loop {
            let b = self.read_u8().await;
            
            match b {
                0x00 => {
                    buff.fill(0x00);
                },
                byte => {
                    buff[i] = byte;
                    i += 1;

                    if i >= DMX_SIZE {
                        // buff.fill(0x00);
                        // i = 0;
                        break
                    }
                }
            }
        }

        // self.sm.rx().dma_pull(self.dma.reborrow(), buff, false)
    }

    // pub async fn read_async(&mut self) {
    //     self.sm.set_enable(false);
    //     self.sm.clear_fifos();
    //     self.sm.restart();
    //     let dma_future = self.sm.read_async(&buffer);
    //     // unsafe { self.sm.exec_jmp(0) };
    //     // self.sm.clear_fifos();
    //     // let d = self.sm.rx().wait_pull().await as u8;
    //     // let a = self.sm.tx().dma_push(self.dma_ch.reborrow(), &[d], false).await;
    // }
}
