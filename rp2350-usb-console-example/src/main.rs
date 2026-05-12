#![no_std]
#![no_main]

// For string formatting.
// The macro for our start-up function
// A shorter alias for the Peripheral Access Crate, which provides low-level
// register access
use rp235x_hal as hal;
use hal::{entry, pac, Clock};

use embedded_hal::delay::DelayNs;
use rp_usb_serial::RpUsbConsole;
use rp_usb_serial::usb_println;

use cortex_m::peripheral::NVIC;

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
#[link_section = ".start_block"]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

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

    //let mut delay = cortex_m::delay::Delay::new(core.PLL_SYS, clocks.system_clock.freq().to_Hz());
    let mut timer = hal::Timer::new_timer0(pac.TIMER0, &mut pac.RESETS, &clocks);
    // The single-cycle I/O block controls our GPIO pins
    // let sio = hal::Sio::new(pac.SIO);

    // 初始化 USB
    RpUsbConsole::init(
        pac.USB,
        pac.USB_DPRAM,
        &mut pac.RESETS,
        clocks.usb_clock,
    );

    // 打开 USB 中断
    unsafe {
    NVIC::unpend(pac::Interrupt::USBCTRL_IRQ);
        NVIC::unmask(pac::Interrupt::USBCTRL_IRQ);
    };
    // 直接打印日志！
    usb_println!("=== RP2350 USB 串口启动成功 ===");
    usb_println!("系统时钟: {} Hz", clocks.system_clock.freq().to_Hz());

    test();

    loop {
        usb_println!("主循环运行中...");
        //delay.delay_ms(1000);
        timer.delay_ms(1000);
    }
}

