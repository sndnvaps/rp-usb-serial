# RP USB Serial Console

[![no_std](https://img.shields.io/badge/no__std-supported-blue.svg)](#)
[![RP2040](https://img.shields.io/badge/target-RP2040-ff69b4.svg)](#supported-platforms)
[![RP2350](https://img.shields.io/badge/target-RP2350-orange.svg)](#supported-platforms)
[![USB CDC](https://img.shields.io/badge/USB-CDC%20Serial-green.svg)](#features)
[![Rust](https://img.shields.io/badge/Rust-embedded-black.svg)](#)

A lightweight `no_std` USB CDC serial console for **RP2040** and **RP2350**.

It provides:

- USB CDC virtual serial output
- formatted logging via `usb_print!` / `usb_println!`
- internal RX/TX buffering
- optional interrupt-driven USB maintenance
- poll-driven receive path suitable for future protocol handling such as **Modbus**

---

## Overview

`rp-usb-serial` is a small embedded Rust library that wraps the RP USB peripheral as a **USB CDC ACM serial device**.

It is designed for:

- debug logging
- serial console interaction
- host-to-device byte streaming
- protocol bring-up and early frame parsing
- RP2040 / RP2350 projects using `no_std`

Compared with a simple "write-only USB log" implementation, this library now includes:

- a **TX queue**
- an **RX queue**
- a `poll()` function that drives:
  - USB protocol progress
  - RX pumping
  - optional echo
  - TX flushing

This makes it more suitable for **interactive testing** and **frame-oriented protocols**.

---

## Features

- `no_std`
- USB CDC ACM virtual serial port
- supports **RP2040**
- supports **RP2350 ARM**
- supports **RP2350 riscv32**
- global singleton-style USB console
- internal TX queue
- internal RX queue
- `usb_print!` / `usb_println!` macros
- `poll()`-driven RX/TX processing
- optional use with `USBCTRL_IRQ`

---

## Supported Platforms

### RP2040
- HAL: `rp2040-hal`
- Architecture: ARM
- USB registers:
  - `pac::USBCTRL_REGS`
  - `pac::USBCTRL_DPRAM`

### RP2350 ARM
- HAL: `rp235x-hal`
- Architecture: ARM
- USB registers:
  - `pac::USB`
  - `pac::USB_DPRAM`

### RP2350 riscv32
- HAL: `rp235x-hal`
- Architecture: `riscv32-*-none-*`
- USB registers:
  - `pac::USB`
  - `pac::USB_DPRAM`

> `rp2040` supports ARM targets only.  
> `rp2350` supports ARM and bare-metal `riscv32`.

---

## Cargo Features

The crate uses feature flags to select the target family.

### Default
By default, the crate targets **RP2040**.

### RP2040
```toml
[dependencies]
rp-usb-serial = "0.3.0"
```
### RP2350

```toml
[dependencies]
rp-usb-serial = { version = "0.3.0", default-features = false, features = ["rp2350"] }
```

Installation
Typical dependencies in your application may look like this:


```toml

[dependencies]
panic-halt = "0.2"
embedded-hal = "1.0.0"

# RP2040:
rp2040-hal = { version = "0.10", features = ["rt"], optional = true }

# RP2350:
rp235x-hal = { version = "0.3", features = ["rt"], optional = true }

rp-usb-serial = "0.3.0"
```

Or for RP2350:

```toml

[dependencies]
panic-halt = "0.2"
embedded-hal = "1.0.0"
rp235x-hal = { version = "0.3", features = ["rt"] }
rp-usb-serial = { version = "0.3.4", default-features = false, features = ["rp2350"] }
```
Adjust versions to match your local toolchain and HAL version.

## Core Behavior
The current library behavior is centered around RpUsbConsole::poll().
```
poll() does all of the following:
Polls the USB device stack
Reads incoming bytes from the USB CDC endpoint
Pushes received bytes into the internal RX queue
Optionally echoes received bytes back to host
Flushes queued TX data
Important
read() does not read directly from the USB hardware endpoint anymore.

Instead:

poll() moves data into the internal RX queue
read() consumes data from that RX queue
So if you do not call poll(), incoming data may never reach read().
```

## Quick Start
1. Import the crate

```rust

use rp_usb_serial::{RpUsbConsole, usb_println};
```
2. Initialize USB

RP2040

```rust

RpUsbConsole::init(
    pac.USBCTRL_REGS,
    pac.USBCTRL_DPRAM,
    &mut pac.RESETS,
    clocks.usb_clock,
);
```
RP2350

```rust

RpUsbConsole::init(
    pac.USB,
    pac.USB_DPRAM,
    &mut pac.RESETS,
    clocks.usb_clock,
);
```

3. Regularly call poll()

```rust

loop {
    RpUsbConsole::poll();
}
```
4. Read received bytes

```rust

let mut buf = [0u8; 64];
let n = RpUsbConsole::read(&mut buf);
if n > 0 {
    // process bytes
}
```
5. Print logs

```rust

usb_println!("USB console ready");
usb_println!("rx len = {}", 10);
```

## API Overview

```rust
  RpUsbConsole::init(...)

  //Initializes the USB bus, CDC serial class, USB device, and global console state.

  RpUsbConsole::poll()

  //Performs USB polling, RX pumping, optional echo, and TX flushing.

  RpUsbConsole::write(data: &[u8])
  //Queues data for transmission and attempts to send it.

  RpUsbConsole::print(s: &str)
  //Prints a string without line ending.

  RpUsbConsole::println(s: &str)
  //Prints a string followed by \r\n.

  RpUsbConsole::fmt_write(args)
  //Internal helper for formatted output.

  RpUsbConsole::read(buf: &mut [u8]) -> usize
  //Reads data from the internal RX queue.
```

### Macros

usb_print!

```rust
usb_print!("value = {}", 42);
```

usb_println!

```rust

usb_println!("System boot");
//These macros ultimately route data into the library TX path.
```
Example: Poll-Driven Echo Test

This is the simplest recommended test mode.


```rust

loop {
    RpUsbConsole::poll();

    let mut buf = [0u8; 64];
    let n = RpUsbConsole::read(&mut buf);

    if n > 0 {
        usb_println!("received {} bytes", n);
    }
}
```
```
What happens
host sends data
poll() reads the USB endpoint
data is pushed into RX queue
current library version may also echo received bytes back
read() lets your application consume the same received bytes
```

Example: USBCTRL_IRQ + Main Loop Logging + Echo

Recommended structure:
```
USBCTRL_IRQ handles low-level USB maintenance
main loop handles logging, parsing, and application logic
main loop may still call poll() as a fallback
Interrupt side
The library already provides the USB interrupt entry for supported targets.
```

Main loop side

```rust

loop {
    // optional fallback poll
    RpUsbConsole::poll();

    let mut buf = [0u8; 64];
    let n = RpUsbConsole::read(&mut buf);

    if n > 0 {
        usb_println!("rx {} bytes", n);
    }
}
```
Do not heavily log inside USBCTRL_IRQ.

Prefer doing logs in the main loop.

Example: RP2350 riscv32 main.rs


```rust


#![no_std]
#![no_main]

use panic_halt as _;

use hal::{entry, pac, Clock};
use rp235x_hal as hal;

use rp_usb_serial::{RpUsbConsole, usb_println};
use hal::xh3irq;
use riscv::interrupt;

const XTAL_FREQ_HZ: u32 = 12_000_000u32;

#[link_section = ".start_block"]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    let clocks = hal::clocks::init_clocks_and_plls(
        XTAL_FREQ_HZ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let mut timer = hal::Timer::new_timer0(pac.TIMER0, &mut pac.RESETS, &clocks);

    RpUsbConsole::init(pac.USB, pac.USB_DPRAM, &mut pac.RESETS, clocks.usb_clock);

    unsafe {
        xh3irq::unmask(pac::Interrupt::USBCTRL_IRQ);
        interrupt::enable();
    }

    usb_println!("RP2350 USB CDC boot");
    usb_println!("system clock = {} Hz", clocks.system_clock.freq().to_Hz());

    let mut rx_buf = [0u8; 64];

    loop {
        // keep this as a fallback even when USB interrupt is enabled
        RpUsbConsole::poll();

        let n = RpUsbConsole::read(&mut rx_buf);
        if n > 0 {
            usb_println!("received {} bytes", n);
        }

        timer.delay_ms(1000);
    }
}
```

Depending on your rp235x-hal version, the IRQ enable API may differ slightly.

```
Internal Design
The global singleton stores:

UsbDevice
SerialPort
TX queue
RX queue
TX path
write() pushes bytes into TX queue
poll() / interrupt processing flushes TX queue to host
RX path
poll() reads from USB CDC endpoint
bytes are stored into RX queue
read() retrieves those bytes from RX queue
This makes the design easier to adapt to:

line-based command parsers
binary packet handling
Modbus-like frame parsing
Echo and Logging
The current library implementation includes an internal auto-echo path in poll().

Meaning
When the host sends data:

bytes are stored into RX queue
bytes may also be queued for TX echo
This is useful for:

terminal echo testing
quick bring-up
host-device interaction debugging
Warning
If you also use usb_println! at the same time, your host terminal may show:

echoed raw input
formatted log lines
protocol payloads
all on the same CDC stream.

That is acceptable during bring-up, but not ideal for strict binary protocols.

Preparing for Modbus
This library layout is a good stepping stone toward Modbus or other frame-based protocols.

Recommended future workflow
keep poll() running regularly
collect bytes using read()
implement frame assembly / timeout logic in main loop
parse protocol payload
send response using write()
Important note
For Modbus or other binary protocols, you should avoid mixing:

auto echo
text logs
binary responses
on the same CDC stream during real protocol validation.

Recommended strategy
use echo during early testing only
reduce text logs when validating protocol frames
eventually let the CDC stream carry only request/response protocol data
USB Descriptor
The library uses different USB identity strings depending on the target.

RP2040
Manufacturer: Raspberry Pi
Product: RP2040
Serial: B8ED93AE
VID:PID = 0x2E8A:0x0003
RP2350
Manufacturer: Raspberry Pi
Product: RP2350
Serial: A74D15D8
VID:PID = 0x2E8A:0x10B0
Notes
This crate is intended for no_std embedded environments.
USB clock initialization must be completed before calling init().
If USB interrupt handling is not fully configured, call poll() regularly in the main loop.
read() consumes bytes from the internal RX queue, not directly from serial.read().
TX/RX queues are bounded; if they overflow, bytes may be dropped.
Avoid heavy formatted logging inside interrupt handlers.
For binary protocol work, avoid mixing logs and protocol traffic on the same CDC channel.
Limitations
Current implementation trade-offs include:

byte-by-byte TX flushing instead of bulk writes
bounded TX/RX queues
global singleton design
log traffic and protocol traffic may share the same CDC stream
These are acceptable for bring-up and light embedded console use, but may require refinement for high-throughput protocol applications.
```

Roadmap

 - configurable echo enable/disable API
 - batch writes for better throughput
 - queue overflow diagnostics
 - protocol-oriented examples
 - Modbus-style frame handling example
 - separate debug/protocol channel strategy examples

## Contributing
Issues and pull requests are welcome.

If you report a platform-specific issue, please include:
```
target chip
architecture
target triple
HAL version
Rust version
minimal reproducible example
expected behavior vs actual behavior
```
## License

MIT
Apache-2.0
MIT OR Apache-2.0


## Acknowledgements
Built on top of the embedded Rust ecosystem:

 - usb-device
 - usbd-serial
 - heapless
 - static_cell
 - rp2040-hal
- rp235x-hal

## Summary
rp-usb-serial is a compact USB CDC console layer for RP2040 and RP2350 embedded projects.

## It supports:

 - logging
 - buffered RX/TX
 - poll-based USB maintenance
 - interrupt-assisted USB handling
 - early-stage protocol bring-up

and is especially useful when evolving from simple text-console debugging toward structured protocol processing such as Modbus.