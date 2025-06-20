# ArceOS

[![CI](https://github.com/arceos-org/arceos/actions/workflows/build.yml/badge.svg?branch=main)](https://github.com/arceos-org/arceos/actions/workflows/build.yml)
[![CI](https://github.com/arceos-org/arceos/actions/workflows/test.yml/badge.svg?branch=main)](https://github.com/arceos-org/arceos/actions/workflows/test.yml)
[![Docs](https://img.shields.io/badge/docs-pages-green)](https://arceos-org.github.io/arceos/)

An experimental modular operating system (or unikernel) written in Rust.

ArceOS was inspired a lot by [Unikraft](https://github.com/unikraft/unikraft).

🚧 Working In Progress.

## Features & TODOs

* [x] Architecture: x86_64, riscv64, aarch64
* [x] Platform: QEMU pc-q35 (x86_64), virt (riscv64/aarch64)
* [x] Multi-thread
* [x] FIFO/RR/CFS scheduler
* [x] VirtIO net/blk/gpu drivers
* [x] TCP/UDP net stack using [smoltcp](https://github.com/smoltcp-rs/smoltcp)
* [x] Synchronization/Mutex
* [x] SMP scheduling with single run queue
* [x] File system
* [ ] Compatible with Linux apps
* [ ] Interrupt driven device I/O
* [ ] Async I/O

## Quick Start

### 1. Install Build Dependencies

Install [cargo-binutils](https://github.com/rust-embedded/cargo-binutils) to use `rust-objcopy` and `rust-objdump` tools:

```bash
cargo install cargo-binutils
```

#### Dependencies for C apps

Install `libclang-dev`:

```bash
sudo apt install libclang-dev
```

Currently, the aarch64 architecture supports u_* and m_* under arceos/tour, but m_3_* requires libclang-*-dev version less than or equal to 16. You can use conda to generate a specified clangdev version for successful compilation. Or use dockerfile directly.

Download & install [musl](https://musl.cc) toolchains:

```bash
# download
wget https://musl.cc/aarch64-linux-musl-cross.tgz
wget https://musl.cc/riscv64-linux-musl-cross.tgz
wget https://musl.cc/x86_64-linux-musl-cross.tgz
# install
tar zxf aarch64-linux-musl-cross.tgz
tar zxf riscv64-linux-musl-cross.tgz
tar zxf x86_64-linux-musl-cross.tgz
# exec below command in bash OR add below info in ~/.bashrc
export PATH=`pwd`/x86_64-linux-musl-cross/bin:`pwd`/aarch64-linux-musl-cross/bin:`pwd`/riscv64-linux-musl-cross/bin:$PATH
```

#### Dependencies for running apps

```bash
# for Debian/Ubuntu
sudo apt-get install qemu-system
```

```bash
# for macos
brew install qemu
```

Other systems and arch please refer to [Qemu Download](https://www.qemu.org/download/#linux)

For qemu version, it is recommended to use version 9.2.0 or later, otherwise unexpected problems may occur.

### 2. Build & Run

```bash
# e.g. build app in arceos directory
make A=path/to/app ARCH=<arch> LOG=<log>
# e.g. run app in arceos directory
make run A=path/to/app ARCH=<arch> LOG=<log>
# e.g. build&run oscamp tour app tour/u_1_0
make pflash_img ARCH=<arch> 
make disk_img ARCH=<arch> 
make run A=tour/u_1_0
# e.g. try to build&run oscamp tour apps
./test_tour.sh
```

Where `path/to/app` is the relative path to the application. Examples applications can be found in the [examples](examples/) directory or the [arceos-apps](https://github.com/arceos-org/arceos-apps) repository.

`<arch>` should be one of `riscv64`, `aarch64`, `x86_64`.

`<log>` should be one of `off`, `error`, `warn`, `info`, `debug`, `trace`.

If the above command does not specify ARCH, it defaults to risc-V64.

More arguments and targets can be found in [Makefile](Makefile).

For example, to run the [httpserver](examples/httpserver/) on `qemu-system-aarch64` with 4 cores and log level `info`:

```bash
make A=examples/httpserver ARCH=aarch64 LOG=info SMP=4 run NET=y
```

Note that the `NET=y` argument is required to enable the network device in QEMU. These arguments (`BLK`, `GRAPHIC`, etc.) only take effect at runtime not build time.


For example, the complete process for tour/m_1_0, aarch64 architecture:
``` bash
make
make ARCH=aarch64
rm -f pflash.img
rm -f disk.img
make pflash_img ARCH=aarch64
make disk_img ARCH=aarch64
make payload ARCH=aarch64
./update_disk.sh ./payload/origin/origin
make run A=tour/m_1_0 ARCH=aarch64 LOG=debug BLK=y
```
For tour/m_1_0,risc-V64 architecture:
``` bash
make
rm -f pflash.img
rm -f disk.img
make pflash_img
make disk_img
make payload
./update_disk.sh ./payload/origin/origin
make run A=tour/m_1_0 LOG=debug BLK=y
```

## Build and Run through Docker
Install [Docker](https://www.docker.com/) in your system.

Then build all dependencies through provided dockerfile:
```bash
docker build -t oscamp -f Dockerfile .
```
Create a container and build/run app:
``` bash
docker run -it --privileged \
    -v ~/oscamp:/oscamp \
    -w /oscamp/arceos \
  oscamp bash
```
"By default, Docker containers are isolated, and for security reasons, they run with a restricted set of Linux Capabilities. The `make disk_img` rule requires the `mount` command, and the `mount` command (as well as `umount`) needs the `CAP_SYS_ADMIN` Linux Capability. By default, Docker containers do not possess this capability, even if you are the `root` user inside the container.

**Solution:**
Use the `--privileged` flag (not recommended for production environments, but convenient for development and debugging).
This flag grants the container almost all Linux Capabilities, including `CAP_SYS_ADMIN`, allowing it to perform operations typically reserved for the `root` user, such as mounting file systems."
# Now build/run app in the container
make A=examples/helloworld ARCH=aarch64 run
## How to write ArceOS apps

You can write and build your custom applications outside the ArceOS source tree.
Examples are given below and in the [app-helloworld](https://github.com/arceos-org/app-helloworld) and [arceos-apps](https://github.com/arceos-org/arceos-apps) repositories.

### Rust

1. Create a new rust package with `no_std` and `no_main` environment.
2. Add `axstd` dependency and features to enable to `Cargo.toml`:

    ```toml
    [dependencies]
    axstd = { path = "/path/to/arceos/ulib/axstd", features = ["..."] }
    # or use git repository:
    # axstd = { git = "https://github.com/arceos-org/arceos.git", features = ["..."] }
    ```

3. Call library functions from `axstd` in your code, just like the Rust [std](https://doc.rust-lang.org/std/) library.
    
    Remember to annotate the `main` function with `#[no_mangle]` (see this [example](examples/helloworld/src/main.rs)).

4. Build your application with ArceOS, by running the `make` command in the application directory:

    ```bash
    # in app directory
    make -C /path/to/arceos A=$(pwd) ARCH=<arch> run
    # more args: LOG=<log> SMP=<smp> NET=[y|n] ...
    ```

    All arguments and targets are the same as above.

### C

1. Create `axbuild.mk` and `features.txt` in your project:

    ```bash
    app/
    ├── foo.c
    ├── bar.c
    ├── axbuild.mk      # optional, if there is only one `main.c`
    └── features.txt    # optional, if only use default features
    ```

2. Add build targets to `axbuild.mk`, add features to enable to `features.txt` (see this [example](examples/httpserver-c/)):

    ```bash
    # in axbuild.mk
    app-objs := foo.o bar.o
    ```

    ```bash
    # in features.txt
    alloc
    paging
    net
    ```

3. Build your application with ArceOS, by running the `make` command in the application directory:

    ```bash
    # in app directory
    make -C /path/to/arceos A=$(pwd) ARCH=<arch> run
    # more args: LOG=<log> SMP=<smp> NET=[y|n] ...
    ```

## How to build ArceOS for specific platforms and devices

Set the `PLATFORM` variable when run `make`:

```bash
# Build helloworld for raspi4
make PLATFORM=aarch64-raspi4 A=examples/helloworld
```

You may also need to select the corrsponding device drivers by setting the `FEATURES` variable:

```bash
# Build the shell app for raspi4, and use the SD card driver
make PLATFORM=aarch64-raspi4 A=examples/shell FEATURES=driver-bcm2835-sdhci
# Build httpserver for the bare-metal x86_64 platform, and use the ixgbe and ramdisk driver
make PLATFORM=x86_64-pc-oslab A=examples/httpserver FEATURES=driver-ixgbe,driver-ramdisk SMP=4
```

## How to reuse ArceOS modules in your own project

```toml
# In Cargo.toml
[dependencies]
axalloc = { git = "https://github.com/arceos-org/arceos.git", tag = "v0.1.0" } # modules/axalloc
axhal = { git = "https://github.com/arceos-org/arceos.git", tag = "v0.1.0" } # modules/axhal
```

## Design

![](doc/figures/ArceOS.svg)
