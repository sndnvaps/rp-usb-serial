#![no_std]

use core::cell::RefCell;
use core::fmt::Write;
use core::sync::atomic::{AtomicBool, Ordering};

use critical_section::Mutex;
use heapless::spsc::Queue;
use static_cell::StaticCell;
use usb_device::prelude::*;
use usb_device::{class_prelude::UsbBusAllocator, device::UsbDeviceState};
use usbd_serial::SerialPort;

#[cfg(feature = "rp2040")]
use rp2040_hal as hal;
#[cfg(feature = "rp2350")]
use rp235x_hal as hal;

use hal::pac;
use hal::usb::UsbBus;

// 由 PAC 提供中断属性宏；在 ARM / RP2350-RISCV 下都统一使用它
#[cfg(any(
    target_arch = "arm",
    all(feature = "rp2350", target_arch = "riscv32", target_os = "none")
))]
use hal::pac::interrupt;

// ==============================
// 配置合法性检查
// ==============================

#[cfg(all(feature = "rp2040", feature = "rp2350"))]
compile_error!("features `rp2040` and `rp2350` cannot be enabled at the same time");

#[cfg(not(any(feature = "rp2040", feature = "rp2350")))]
compile_error!("either feature `rp2040` or `rp2350` must be enabled");

//#[cfg(all(feature = "rp2040", not(target_arch = "arm")))]
//compile_error!("`rp2040` only supports ARM targets");

#[cfg(all(
    feature = "rp2350",
    not(any(target_arch = "arm", all(target_arch = "riscv32", target_os = "none")))
))]
compile_error!("`rp2350` supports ARM or bare-metal riscv32 targets only");

// ==============================

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
        #[cfg(feature = "rp2040")] usbctrl_dpram: pac::USBCTRL_DPRAM,
        #[cfg(feature = "rp2350")] usbctrl_dpram: pac::USB_DPRAM,
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

    pub fn poll() {
        Self::force_poll();
        Self::flush_tx();
    }

    /// 强制轮询 USB 协议
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

    /// 入队 -> 立即尝试发送 -> poll
    pub fn write(data: &[u8]) {
        if !USB_INIT.load(Ordering::SeqCst) {
            return;
        }

        critical_section::with(|cs| {
            let mut binding = USB_CONSOLE.borrow(cs).borrow_mut();
            if let Some(usb) = binding.as_mut() {
                for &b in data {
                    let _ = usb.tx_buf.enqueue(b);
                }

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

// ==============================
// 中断公共处理逻辑
// ==============================

#[inline(always)]
fn usbctrl_irq_impl() {
    RpUsbConsole::force_poll();
    RpUsbConsole::flush_tx();
}

// ARM: RP2040 / RP2350-ARM
#[cfg(target_arch = "arm")]
#[interrupt]
fn USBCTRL_IRQ() {
    usbctrl_irq_impl();
}

// RISC-V: RP2350-Hazard3
#[cfg(all(feature = "rp2350", target_arch = "riscv32", target_os = "none"))]
#[no_mangle]
pub extern "C" fn USBCTRL_IRQ() {
    usbctrl_irq_impl();
}

// ==============================
// 打印宏
// ==============================

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
