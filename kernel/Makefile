ARCH ?= riscv64
MODE ?= release
USERMODE ?= release
SMP ?= 4

OBJDUMP ?= rust-objdump
OBJCOPY ?= rust-objcopy
GDB ?= riscv64-unknown-elf-gdb

ifeq ($(ARCH), riscv64)
target := riscv64imac-unknown-none-elf
else
$(error Unsupported architecture $(ARCH))
endif

qemu := qemu-system-$(ARCH)
build_path := target/$(target)/$(MODE)
kernel := $(build_path)/maturin
kernel_img := $(build_path)/maturin.img
fs_img := ../user/target/$(target)/$(USERMODE)/fs.img

build_args := --target $(target)
ifeq ($(MODE), release)
build_args += --release
endif

qemu_args := -nographic -smp $(SMP)
ifeq ($(ARCH), riscv64)
qemu_args += \
	-machine virt \
	-bios default \
	-device loader,file=$(kernel_img),addr=0x80200000 \
	-drive file=$(fs_img),if=none,format=raw,id=x0 \
    -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0

endif

.PHONY: build kernel run qemu asm clean

build: $(kernel_img) fs-img

fs-img: $(APPS)
	@cd ../user && make build
	@rm -f $(fs_img)
	@cd ../easy-fs-fuse && cargo run --release -- -s ../user/src/bin/ -t ../user/target/$(target)/$(USERMODE)/


kernel:
	@echo Building maturin kernel
	cargo build $(build_args)

$(kernel_img): kernel
	$(OBJCOPY) $(kernel) --strip-all -O binary $@

run: build qemu

qemu:
	$(qemu) $(qemu_args)

asm:
	$(OBJDUMP) -ld $(kernel) > dbg.S

gdb-runner:
	$(qemu) $(qemu_args) -s -S

gdb-listener:
	$(GDB) -ex 'file $(kernel)'  -ex 'set arch riscv:rv64' -ex 'target remote localhost:1234'

clean:
	cargo clean
