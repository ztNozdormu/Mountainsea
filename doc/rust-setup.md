---
title: Installation
---

This page will guide you through the steps needed to prepare a computer for development with the
Substrate Node Template. Since Substrate is built with
[the Rust programming language](https://www.rust-lang.org/), the first thing you will need to do is
prepare the computer for Rust development - these steps will vary based on the computer's operating
system. Once Rust is configured, you will use its toolchains to interact with Rust projects; the
commands for Rust's toolchains will be the same for all supported, Unix-based operating systems.

## Unix-Based Operating Systems

Substrate development is easiest on Unix-based operating systems like macOS or Linux. The examples
in the Substrate [Tutorials](https://substrate.dev/tutorials) and [Recipes](https://substrate.dev/recipes/)
use Unix-style terminals to demonstrate how to interact with Substrate from the command line.

### macOS

Open the Terminal application and execute the following commands:

```bash
# Install Homebrew if necessary https://brew.sh/
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/master/install.sh)"

# Make sure Homebrew is up-to-date, install openssl and cmake
brew update
brew install openssl cmake
```

### Ubuntu/Debian

Use a terminal shell to execute the following commands:

```bash
sudo apt update
# May prompt for location information
sudo apt install -y cmake pkg-config libssl-dev git build-essential clang libclang-dev curl
```

### Arch Linux

Run these commands from a terminal:

```bash
pacman -Syu --needed --noconfirm cmake gcc openssl-1.0 pkgconf git clang
export OPENSSL_LIB_DIR="/usr/lib/openssl-1.0"
export OPENSSL_INCLUDE_DIR="/usr/include/openssl-1.0"
```

## Rust Developer Environment

This project uses [`rustup`](https://rustup.rs/) to help manage the Rust toolchain. First install
and configure `rustup`:

```bash
# Install
curl https://sh.rustup.rs -sSf | sh
# Configure
source ~/.cargo/env
```

Finally, configure the Rust toolchain to default to the latest stable version:

```bash
rustup default stable
```

## Build the Project

Now that the standard Rust environment is configured, use the
[included Makefile](../README.md#makefile) to install the project-specific toolchains and build the
project.

### substrate框架windows开发环境

   #### windows10环境LLVM的编译
   1. 准备工作
    1. 安装Visual Studio Comminuty 201X版本IDE、 安装windows SDK、安装cmake、安装python2.7版本
       clang的安装(https://www.cnblogs.com/FrankOu/p/14215850.html)
    2. 下载LLVM源代码 、clang源代码
    3. 解压LLVM到自定义目录，解压clang源代码到clang文件夹，并将clang文件夹移动到LLVM源代码tools文件夹中
    4. LLVM源目录下执行cmake .操作，生成llvm.sln文件
    5.  双击llvm.sln选择用visual studio打开，加载完成后生成菜单下拉选择 ALL BUILD。
    6. 生成完成后再源代码bin目录下，配置环境变量

  #### 参考文章：
      https://blog.csdn.net/wgx571859177/article/details/80376134
      https://blog.csdn.net/u013195275/article/details/106871312
      注意：除以上安装程序以外，还需要单独定义配置环境变量LIBCLANG_PATH （指向libclang.dll clang.dll文件）
  2. 前端环境安装：
      1. nodejs安装  https://www.cnblogs.com/qiangyuzhou/p/10836561.html
      2.  yarn安装    https://blog.csdn.net/hq422/article/details/89671391
  #### windows10安装ubuntu子系统
     1. https://blog.csdn.net/zhouzme/article/details/78780479    
  #### ubuntu环境搭建
    1.  安装rust环境
     - https://www.cnblogs.com/honyer/p/11877145.html
    2. vscode远程编译
       1. 插件安装、使用
          - https://www.cnblogs.com/chnmig/p/12160248.html    
    3. mclone代码加速拉取插件安装
       - https://github.com/nulastudio/mclone

###  substrate框架基础知识
   1. Substrate存储数据类型概览
      - https://whisperd.tech/post/substrate_storage_data_type/#%E5%8D%95%E5%80%BC%E7%B1%BB%E5%9E%8B
### substrate智能合约
   1. 智能合约书文档
      - 仓库地址是这个：https://github.com/patractlabs/substrate-contracts-book/
   2. 智能合约部署
      - https://patractlabs.github.io/substrate-contracts-book/zh_CN/index.html      
### 亚洲开发区技术讨论论坛
     https://github.com/ParityAsia/substrate-newsletter/discussions


### 黑客松项目

   1. 项目案例
     - 文档
    https://github.com/w3f-community/nft_parachain/blob/master/doc/hackusama_walk_through.md     
   2. 代码
    https://uniqueapps.usetech.com/#/nft 

