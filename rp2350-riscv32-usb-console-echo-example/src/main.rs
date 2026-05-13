#![no_std]
#![no_main]

use panic_halt as _;
use rp235x_hal as hal;
use hal::{entry, pac, Clock};

use embedded_hal::delay::DelayNs;
use rp_usb_serial::{usb_println, RpUsbConsole};

// RP2350 RISC-V 中断相关
use hal::xh3irq;
use riscv::interrupt;

// 外部晶振频率
const XTAL_FREQ_HZ: u32 = 12_000_000u32;

/// Boot image definition for RP2350
#[link_section = ".start_block"]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

fn test_log() {
    usb_println!("这是在外部函数测试");
}

#[entry]
fn main() -> ! {
    // 1. 获取外设
    let mut pac = pac::Peripherals::take().unwrap();

    // 2. 初始化 watchdog
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    // 3. 初始化时钟
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

    // 4. 初始化定时器
    let mut timer = hal::Timer::new_timer0(pac.TIMER0, &mut pac.RESETS, &clocks);

    // 5. 初始化 USB CDC
    RpUsbConsole::init(pac.USB, pac.USB_DPRAM, &mut pac.RESETS, clocks.usb_clock);

    // 6. 使能 USB 中断
    //
    // 注意：
    // - 这里仅演示常见做法
    // - 如果你的 rp235x-hal 版本 API 有差异，需要按本地版本微调
    unsafe {
        xh3irq::unmask(pac::Interrupt::USBCTRL_IRQ);
        interrupt::enable();
    }

    // 7. 启动日志
    //
    // 由于此时主机可能还没完成枚举，
    // 前几条日志可能会延迟显示，这是正常现象。
    usb_println!("================================");
    usb_println!("RP2350 USB CDC boot");
    usb_println!("system clock = {} Hz", clocks.system_clock.freq().to_Hz());
    usb_println!("chip         = RP2350");
    usb_println!("arch         = riscv32");
    usb_println!("================================");

    test_log();

    let mut rx_buf = [0u8; 64];
    let mut tick: u32 = 0;

    loop {
        // 8. 主循环里再 poll 一次，作为兜底
        //
        // 即使 USBCTRL_IRQ 已经在工作，
        // 保留这一句也会让系统更稳一些。
        RpUsbConsole::poll();

        // 9. 从内部 RX 缓冲读取数据
        //
        // 注意：
        // 当前库的新版本里，read() 读的是内部 rx_buf，
        // 而不是直接读 USB 硬件端点。
        let n = RpUsbConsole::read(&mut rx_buf);

        if n > 0 {
            // 自动回显已经在 poll() 中完成
            // 这里仅打印日志，不重复发送原始数据
            usb_println!("收到 {} 字节数据", n);

            // 如果你想调试内容，可以打印十六进制或 ASCII
            // 但注意：后续做 Modbus 时，不建议频繁往同一 CDC 通道打文本日志
            for &b in &rx_buf[..n] {
                usb_println!("RX byte = 0x{:02X}", b);
            }
        }

        // 10. 周期性输出运行日志
        tick = tick.wrapping_add(1);
        if tick % 5 == 0 {
            usb_println!("主循环运行中... tick={}", tick);
        }

        timer.delay_ms(1000);
    }
}
