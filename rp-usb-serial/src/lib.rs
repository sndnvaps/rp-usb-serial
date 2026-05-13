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
/*
//mini example for rp2040
#![no_std]
#![no_main]
// For string formatting.
// The macro for our start-up function
// A shorter alias for the Peripheral Access Crate, which provides low-level
// register access
use hal::{entry, pac, Clock};
use rp2040_hal as hal;

use rp_usb_serial::RpUsbConsole;
use rp_usb_serial::usb_println;

// Ensure we halt the program on panic (if we don't mention this crate it won't
// be linked)
use panic_halt as _;

/// External high-speed crystal on the Raspberry Pi Pico board is 12 MHz. Adjust
/// if your board has a different frequency
const XTAL_FREQ_HZ: u32 = 12_000_000u32;

/// The linker will place this boot block at the start of our program image. We
/// need this to help the ROM bootloader get our code up and running.
/// Note: This boot block is not necessary when using a rp-hal based BSP
/// as the BSPs already perform this step.
#[link_section = ".boot2"]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_GENERIC_03H;

/// Entry point to our bare-metal application.
///
/// The `#[entry]` macro ensures the Cortex-M start-up code calls this function
/// as soon as all global variables are initialised.
///
/// The function configures the RP2040 peripherals,
/// gets a handle on the I2C peripheral,
/// initializes the SSD1306 driver, initializes the text builder
/// and then draws some text on the display.
///
///
fn test () {
    //this is the test for usb_println
    usb_println!("这是在外部函数测试");
}
#[entry]
fn main() -> ! {
    // Grab our singleton objects
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    // Set up the watchdog driver - needed by the clock setup code
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    // Configure the clocks
    //
    // The default is to generate a 125 MHz system clock
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

    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());
    // The single-cycle I/O block controls our GPIO pins
    // let sio = hal::Sio::new(pac.SIO);

    // 初始化 USB
    RpUsbConsole::init(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        &mut pac.RESETS,
        clocks.usb_clock,
    );

    // 打开 USB 中断
    unsafe {
        pac::NVIC::unpend(pac::Interrupt::USBCTRL_IRQ);
        pac::NVIC::unmask(pac::Interrupt::USBCTRL_IRQ);
    };
    // 直接打印日志！
    usb_println!("=== RP2040 USB 串口启动成功 ===");
    usb_println!("系统时钟: {} Hz", clocks.system_clock.freq().to_Hz());

    test();

    loop {
        usb_println!("主循环运行中...");
        delay.delay_ms(1000);
    }
}
*/

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

#[cfg(all(feature = "rp2040", not(target_arch = "arm")))]
compile_error!("`rp2040` only supports ARM targets");

#[cfg(all(
    feature = "rp2350",
    not(any(target_arch = "arm", all(target_arch = "riscv32", target_os = "none")))
))]
compile_error!("`rp2350` supports ARM or bare-metal riscv32 targets only");

// ==============================

const TX_BUF_SIZE: usize = 1024;
const RX_BUF_SIZE: usize = 1024;

/// 是否自动回显主机发来的数据
/// 为后续 Modbus 调试保留：现在打开，后面可直接改成 false
const AUTO_ECHO: bool = true;

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
    rx_buf: Queue<u8, RX_BUF_SIZE>,
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
                rx_buf: Queue::new(),
            }));
        });

        USB_INIT.store(true, Ordering::SeqCst);
    }

    /// 轮询 USB：
    /// 1. 推进 USB 协议
    /// 2. 读取主机输入
    /// 3. 放入 RX 缓冲
    /// 4. 可选回显
    /// 5. 刷新 TX 发送
    pub fn poll() {
        Self::force_poll();
        Self::pump_rx_echo();
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

    /// 从 USB 端点读取数据，写入 RX 队列，并按需回显到 TX 队列
    fn pump_rx_echo() {
        if !USB_INIT.load(Ordering::SeqCst) {
            return;
        }

        critical_section::with(|cs| {
            let mut opt = USB_CONSOLE.borrow(cs).borrow_mut();
            let Some(usb) = opt.as_mut() else { return };

            if usb.device.state() != UsbDeviceState::Configured {
                return;
            }

            let mut temp = [0u8; 64];

            loop {
                match usb.serial.read(&mut temp) {
                    Ok(0) => break,
                    Ok(n) => {
                        for &b in &temp[..n] {
                            // 保存到 RX 缓冲，供上层协议读取
                            let _ = usb.rx_buf.enqueue(b);

                            // 自动回显：收到什么就回什么
                            if AUTO_ECHO {
                                let _ = usb.tx_buf.enqueue(b);
                            }
                        }
                    }
                    Err(usb_device::UsbError::WouldBlock) => break,
                    Err(_) => break,
                }
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

    /// 写数据到发送队列，并立即尝试发送
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
            }
        });

        // 发送后顺便做一次完整轮询
        Self::poll();
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

    /// 从内部 RX 缓冲读取数据
    /// 注意：现在 read() 不再直接访问 USB 端点，而是读取 poll() 已缓存的数据
    pub fn read(buf: &mut [u8]) -> usize {
        if !USB_INIT.load(Ordering::SeqCst) {
            return 0;
        }

        critical_section::with(|cs| {
            let mut opt = USB_CONSOLE.borrow(cs).borrow_mut();
            let Some(usb) = opt.as_mut() else { return 0 };

            let mut n = 0;
            while n < buf.len() {
                match usb.rx_buf.dequeue() {
                    Some(b) => {
                        buf[n] = b;
                        n += 1;
                    }
                    None => break,
                }
            }
            n
        })
    }
}

// ==============================
// 中断公共处理逻辑
// ==============================

#[inline(always)]
fn usbctrl_irq_impl() {
    RpUsbConsole::poll();
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
