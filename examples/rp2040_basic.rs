#![no_std]
#![no_main]
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

    // 初始化 USB
    RpUsbConsole::init(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        &mut pac.RESETS,
        clocks.usb_clock,
    );

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