# RP USB Serial Console  
# RP 系列 USB CDC 串口控制台

[![no_std](https://img.shields.io/badge/no__std-supported-blue.svg)](#)
[![RP2040](https://img.shields.io/badge/target-RP2040-ff69b4.svg)](#supported-platforms--支持平台)
[![RP2350](https://img.shields.io/badge/target-RP2350-orange.svg)](#supported-platforms--支持平台)
[![USB CDC](https://img.shields.io/badge/USB-CDC%20Serial-green.svg)](#features--功能特性)
[![Rust](https://img.shields.io/badge/Rust-embedded-black.svg)](#)

A lightweight `no_std` USB CDC serial console for **RP2040** and **RP2350**.  
一个轻量级的 `no_std` USB CDC 虚拟串口控制台模块，支持 **RP2040** 和 **RP2350**。

It provides a simple global USB serial interface for logging, formatted printing, and host-device communication.  
它提供了一个简单的全局 USB 串口接口，可用于日志输出、格式化打印以及主机与设备之间的数据通信。

---

## Table of Contents  
## 目录

- [Features / 功能特性](#features--功能特性)
- [Supported Platforms / 支持平台](#supported-platforms--支持平台)
- [Installation / 安装方式](#installation--安装方式)
- [Cargo Features / Cargo 特性](#cargo-features--cargo-特性)
- [Quick Start / 快速开始](#quick-start--快速开始)
- [Examples / 示例](#examples--示例)
- [API Overview / API 概览](#api-overview--api-概览)
- [How It Works / 工作原理](#how-it-works--工作原理)
- [USB Descriptor / USB 设备描述信息](#usb-descriptor--usb-设备描述信息)
- [Notes / 注意事项](#notes--注意事项)
- [Roadmap / 路线图](#roadmap--路线图)
- [License / 许可证](#license--许可证)

---

## Features  
## 功能特性

- USB CDC ACM virtual serial port  
  USB CDC ACM 虚拟串口

- Supports both **RP2040** and **RP2350** via Cargo features  
  通过 Cargo feature 同时支持 **RP2040** 和 **RP2350**

- `no_std` compatible  
  兼容 `no_std`

- Global singleton-style USB console  
  全局单例式 USB 控制台

- Buffered TX queue for outgoing data  
  内置发送缓冲队列

- Automatic USB polling and TX flushing  
  自动轮询 USB 协议并自动刷新发送缓冲

- Formatted output support  
  支持格式化输出

- Easy-to-use APIs:
  - `init()`
  - `write()`
  - `read()`
  - `print()`
  - `println()`

- Convenience macros:
  - `usb_print!`
  - `usb_println!`

---

## Supported Platforms  
## 支持平台

### RP2040
- HAL: `rp2040-hal`
- USB registers:
  - `pac::USBCTRL_REGS`
  - `pac::USBCTRL_DPRAM`

### RP2350
- HAL: `rp235x-hal`
- USB registers:
  - `pac::USB`
  - `pac::USB_DPRAM`

> This implementation is intended for the ARM-based embedded Rust workflow.  
> 此实现面向 ARM 风格的嵌入式 Rust 工作流。

---

## How to use it
## 如何使用

### Example for rp2040 arm
### RP2040平台使用例子
```toml
#cargo.toml
#default features is rp2040 
rp-usb-serial = "0.3.0"
```
[Example for rp2040 arm](https://github.com/sndnvaps/rp-usb-serial/tree/main/rp2040-usb-console-example)

### Example for rp2350 arm
### RP2350平台使用例子
```toml
#cargo.toml
rp-usb-serial = {version = "0.3.0",default-features = false, features = [ "rp2350"]}
```

[Example for rp2350 arm](https://github.com/sndnvaps/rp-usb-serial/tree/main/rp2350-usb-console-example)


### Example for rp2350 riscv32
### RP2350平台使用例子
```toml
#cargo.toml
rp-usb-serial = {version = "0.3.0",default-features = false, features = [ "rp2350"]}
```

[Example for rp2350 riscv32](https://github.com/sndnvaps/rp-usb-serial/tree/main/rp2350-riscv32-usb-console-example)

