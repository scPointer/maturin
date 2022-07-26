ARCH ?= riscv64
MODE ?= debug
USERMODE ?= release
PLATFORM ?= qemu
SMP ?= 4
MACHINE ?= virt 

OBJDUMP ?= rust-objdump
OBJCOPY ?= rust-objcopy
GDB ?= riscv64-unknown-elf-gdb

DISK_DIR ?= libc
# BOOTLOADER := ../bin/fw_jump.bin
export PLATFORM

ifeq ($(ARCH), riscv64)
target := riscv64imac-unknown-none-elf
#target := riscv64.json
else
$(error Unsupported architecture $(ARCH))
endif

qemu := qemu-system-$(ARCH)
build_path := target/$(target)/$(MODE)
kernel := $(build_path)/maturin
kernel_img := $(build_path)/maturin.img
fs_img := ../user/target/$(target)/$(USERMODE)/fs.img
fat_fs_img := ../user/target/$(target)/$(USERMODE)/fat.img
testcases_img := ../fat.img

build_args := --target $(target)
ifeq ($(MODE), release)
build_args += --release
endif

qemu_args := -nographic -smp $(SMP) -m 8G
ifeq ($(ARCH), riscv64)
qemu_args += \
	-machine $(MACHINE) \
	-bios default \
	-kernel $(kernel_img)
endif

ifeq ($(MACHINE), virt)
qemu_args += -drive file=$(fs_img),if=none,format=raw,id=x0
qemu_args += -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0
endif

.PHONY: build kernel run qemu asm clean

#build: $(kernel_img) easy-fs-img
build: $(kernel_img)
	cp $(kernel_img) ../os.bin
easy-fs-img:
	@cd ../user && make build
	@rm -f $(fs_img)
	@cd ../dependencies/easy-fs-fuse && cargo run --release -- -s ../../user/src/bin/ -t ../../user/target/$(target)/$(USERMODE)/

fat-fs-img:
	@rm -f $(fat_fs_img)
	@cd ../fs-init && cargo run --release -- -s ../user/src/bin/ -t ../user/target/$(target)/$(USERMODE)/ -o ../user/target/

testcases-img:
	@rm -f $(testcases_img)
	@cd ../fs-init && cargo run --release -- -b -s ../oscomp_testcases/$(DISK_DIR)/ -t ../oscomp_testcases/$(DISK_DIR)/ -o ../

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

doc:
	cargo doc $(build_args)
