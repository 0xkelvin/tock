// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use core::fmt::Write;
use core::panic::PanicInfo;
use core::ptr::addr_of;
use core::ptr::addr_of_mut;

use kernel::debug;
use kernel::debug::IoWrite;
use kernel::hil::led;
use kernel::hil::uart;
use kernel::hil::uart::Configure;

use stm32f412g::chip_specs::Stm32f412Specs;
use stm32f412g::gpio::PinId;

use crate::CHIP;
use crate::PROCESSES;
use crate::PROCESS_PRINTER;

/// Writer is used by kernel::debug to panic message to the serial port.
pub struct Writer {
    initialized: bool,
}

/// Global static for debug writer
pub static mut WRITER: Writer = Writer { initialized: false };

impl Writer {
    /// Indicate that USART has already been initialized. Trying to double
    /// initialize USART2 causes STM32F412G to go into in in-deterministic state.
    pub fn set_initialized(&mut self) {
        self.initialized = true;
    }
}

impl Write for Writer {
    fn write_str(&mut self, s: &str) -> ::core::fmt::Result {
        self.write(s.as_bytes());
        Ok(())
    }
}

impl IoWrite for Writer {
    fn write(&mut self, buf: &[u8]) -> usize {
        let rcc = stm32f412g::rcc::Rcc::new();
        let clocks: stm32f412g::clocks::Clocks<Stm32f412Specs> =
            stm32f412g::clocks::Clocks::new(&rcc);
        let uart = stm32f412g::usart::Usart::new_usart2(&clocks);

        if !self.initialized {
            self.initialized = true;

            let _ = uart.configure(uart::Parameters {
                baud_rate: 115200,
                stop_bits: uart::StopBits::One,
                parity: uart::Parity::None,
                hw_flow_control: false,
                width: uart::Width::Eight,
            });
        }

        for &c in buf {
            uart.send_byte(c);
        }

        buf.len()
    }
}

/// Panic handler.
#[no_mangle]
#[panic_handler]
pub unsafe fn panic_fmt(info: &PanicInfo) -> ! {
    // User LD2 is connected to PB07
    // Have to reinitialize several peripherals because otherwise can't access them here.
    let rcc = stm32f412g::rcc::Rcc::new();
    let clocks: stm32f412g::clocks::Clocks<Stm32f412Specs> = stm32f412g::clocks::Clocks::new(&rcc);
    let syscfg = stm32f412g::syscfg::Syscfg::new(&clocks);
    let exti = stm32f412g::exti::Exti::new(&syscfg);
    let pin = stm32f412g::gpio::Pin::new(PinId::PE02, &exti);
    let gpio_ports = stm32f412g::gpio::GpioPorts::new(&clocks, &exti);
    pin.set_ports_ref(&gpio_ports);
    let led = &mut led::LedHigh::new(&pin);
    let writer = &mut *addr_of_mut!(WRITER);

    debug::panic(
        &mut [led],
        writer,
        info,
        &cortexm4::support::nop,
        &*addr_of!(PROCESSES),
        &*addr_of!(CHIP),
        &*addr_of!(PROCESS_PRINTER),
    )
}
