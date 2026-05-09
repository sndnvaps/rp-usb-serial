#![no_std]

use core::cell::RefCell;
use core::fmt::Write;
use core::sync::atomic::{AtomicBool, Ordering};

use critical_section::Mutex;
#[cfg(feature = "rp2040")]
use rp2040_hal as hal;
#[cfg(feature = "rp2350")]
use rp235x_hal as hal;

use hal::pac;
use hal::pac::interrupt;
use hal::usb::UsbBus;

use heapless::spsc::Queue;

use static_cell::StaticCell;
use usb_device::prelude::*;
use usb_device::{class_prelude::UsbBusAllocator, device::UsbDeviceState};
use usbd_serial::SerialPort;

const TX_BUF_SIZE: usize = 1024;

static USB_ALLOCATOR: StaticCell<UsbBusAllocator<UsbBus>> = StaticCell::new();
static USB_CONSOLE: Mutex<RefCell<Option<RpUsbConsole>>> = Mutex::new(RefCell::new(None));
static USB_INIT: AtomicBool = AtomicBool::new(false);

#[cfg(feature = "rp2040")]
const USB_CHIP_NAME: &str = "RP2040";
#[cfg(feature = "rp2040")]
const SERIAL_NUM: &str = "B8ED93AE";
#[cfg(feature = "rp2040")]
const USB_VID_PID: (u16, u16) = (0x2E8A, 0x0003);

#[cfg(feature = "rp2350")]
const USB_CHIP_NAME: &str = "RP2350";
#[cfg(feature = "rp2350")]
const SERIAL_NUM: &str = "A74D15D8";
#[cfg(feature = "rp2350")]
const USB_VID_PID: (u16, u16) = (0x2E8A, 0x10B0);

pub struct RpUsbConsole {
    device: UsbDevice<'static, UsbBus>,
    serial: SerialPort<'static, UsbBus>,
    tx_buf: Queue<u8, TX_BUF_SIZE>,
}

impl RpUsbConsole {
    pub fn init(
        #[cfg(feature = "rp2040")] usbctrl_reg: pac::USBCTRL_REGS,
        #[cfg(feature = "rp2350")] usbctrl_reg: pac::USB,
        #[cfg(feature = "rp2350")] usbctrl_dpram: pac::USB_DPRAM,
        #[cfg(feature = "rp2040")] usbctrl_dpram: pac::USBCTRL_DPRAM,

        usb_reset: &mut pac::RESETS,
        clocks: hal::clocks::UsbClock,
    ) {
        if USB_INIT.load(Ordering::SeqCst) {
            return;
        }

        let usb_bus = UsbBus::new(usbctrl_reg, usbctrl_dpram, clocks, true, usb_reset);
        let alloc = USB_ALLOCATOR.init(UsbBusAllocator::new(usb_bus));

        let serial = SerialPort::new(alloc);
        let device = UsbDeviceBuilder::new(alloc, UsbVidPid(USB_VID_PID.0, USB_VID_PID.1))
            .strings(&[StringDescriptors::default()
                .manufacturer("Raspberry Pi")
                .product(USB_CHIP_NAME)
                .serial_number(SERIAL_NUM)])
            .unwrap()
            .device_class(2)
            .build();

        critical_section::with(|cs| {
            USB_CONSOLE.borrow(cs).replace(Some(Self {
                device,
                serial,
                tx_buf: Queue::new(),
            }));
        });

        USB_INIT.store(true, Ordering::SeqCst);
    }

    /// 强制轮询USB协议（自动执行）
    #[inline(always)]
    fn force_poll() {
        if !USB_INIT.load(Ordering::SeqCst) {
            return;
        }
        critical_section::with(|cs| {
            let mut opt = USB_CONSOLE.borrow(cs).borrow_mut();
            if let Some(usb) = opt.as_mut() {
                usb.device.poll(&mut [&mut usb.serial]);
            }
        });
    }

    /// 自动发送缓冲数据
    fn flush_tx() {
        if !USB_INIT.load(Ordering::SeqCst) {
            return;
        }
        critical_section::with(|cs| {
            let mut opt = USB_CONSOLE.borrow(cs).borrow_mut();
            let Some(usb) = opt.as_mut() else { return };
            if usb.device.state() != UsbDeviceState::Configured {
                return;
            }
            while let Some(b) = usb.tx_buf.dequeue() {
                if usb.serial.write(&[b]).is_err() {
                    let _ = usb.tx_buf.enqueue(b);
                    break;
                }
            }
        });
    }

    // =========================================================================
    // ✅ 你要的逻辑：入队 → 立即发送 → 自动poll → 不需要电脑输入触发
    // =========================================================================
    pub fn write(data: &[u8]) {
        if !USB_INIT.load(Ordering::SeqCst) {
            return;
        }

        critical_section::with(|cs| {
            let mut binding = USB_CONSOLE.borrow(cs).borrow_mut();
            if let Some(usb) = binding.as_mut() {
                // 1. 先入队
                for &b in data {
                    let _ = usb.tx_buf.enqueue(b);
                }

                // 2. 立即发送（不等中断、不等缓冲）
                if usb.device.state() == UsbDeviceState::Configured {
                    while let Some(b) = usb.tx_buf.dequeue() {
                        if usb.serial.write(&[b]).is_err() {
                            let _ = usb.tx_buf.enqueue(b);
                            break;
                        }
                    }
                }
            }
        });

        // 3. 自动执行协议轮询，不需要外部触发
        Self::force_poll();
    }

    pub fn print(s: &str) {
        Self::write(s.as_bytes());
    }

    pub fn println(s: &str) {
        Self::print(s);
        Self::write(b"\r\n");
    }

    pub fn fmt_write(args: core::fmt::Arguments<'_>) {
        struct Wrapper;
        impl Write for Wrapper {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                RpUsbConsole::print(s);
                Ok(())
            }
        }
        let _ = Wrapper.write_fmt(args);
    }

    pub fn read(buf: &mut [u8]) -> usize {
        if !USB_INIT.load(Ordering::SeqCst) {
            return 0;
        }
        critical_section::with(|cs| {
            let mut opt = USB_CONSOLE.borrow(cs).borrow_mut();
            if let Some(usb) = opt.as_mut() {
                usb.serial.read(buf).unwrap_or(0)
            } else {
                0
            }
        })
    }
}

// 中断里只做协议poll，保证USB不死
#[cfg(feature = "rp2040")]
#[interrupt]
fn USBCTRL_IRQ() {
    RpUsbConsole::force_poll();
    RpUsbConsole::flush_tx();
}

// 中断里只做协议poll，保证USB不死
/*
#[cfg(feature = "rp2350")]
interrupt!(USBCTRL_IRQ, usbctrl_irq);
#[cfg(feature = "rp2350")]
fn usbctrl_irq() {
    RpUsbConsole::force_poll();
    RpUsbConsole::flush_tx();
}
*/
#[cfg(feature = "rp2350")]
#[interrupt]
fn USBCTRL_IRQ() {
    RpUsbConsole::force_poll();
    RpUsbConsole::flush_tx();
}

// 打印宏
#[macro_export]
macro_rules! usb_print {
    ($($tt:tt)*) => {
        $crate::RpUsbConsole::fmt_write(format_args!($($tt)*));
    };
}

#[macro_export]
macro_rules! usb_println {
    ($($tt:tt)*) => {
        $crate::RpUsbConsole::fmt_write(format_args!($($tt)*));
        $crate::RpUsbConsole::write(b"\r\n");
    };
}
