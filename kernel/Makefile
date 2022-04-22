ARCH ?= riscv64
MODE ?= release
SMP ?= 5

OBJDUMP ?= rust-objdump
OBJCOPY ?= rust-objcopy
PLATFORM ?= qemu
MACHINE := virt
BOOTLOADER := ../bin/fw_jump.bin
ifeq ($(ARCH), riscv64)
target := riscv64imac-unknown-none-elf
else
$(error Unsupported architecture $(ARCH))
endif

qemu := qemu-system-$(ARCH)
build_path := target/$(target)/$(MODE)
kernel := $(build_path)/maturin
kernel_img := $(build_path)/maturin.bin

build_args := --target $(target)
ifeq ($(MODE), release)
build_args += --release
endif

export PLATFORM

qemu_args := -nographic -smp $(SMP) -m 2G
ifeq ($(ARCH), riscv64)
qemu_args += \
	-machine $(MACHINE) \
	-bios  default \
	-kernel $(kernel)
endif

.PHONY: build kernel run qemu asm clean

build: $(kernel_img)

kernel:
	@echo Building maturin kernel
	cargo build $(build_args)

$(kernel_img): kernel
	$(OBJCOPY) --binary-architecture=$(ARCH) $(kernel) --strip-all -O binary $@

fu740:
	gzip -9 -cvf $(build_path)/$(kernel_img) > $(build_path)/maturin.bin.gz
	mkimage -A riscv -O linux -C gzip -T kernel -a 80200000 -e 80200000 -n "maturin-fu740" -d $(build_path)/maturin.bin.gz $(build_path)/maturin-fu740

run: build qemu

gdb := riscv64-unknown-elf-gdb

debug:build
	$(qemu) $(qemu_args) -S -gdb tcp::15234 &
	@sleep 1
	$(gdb)

qemu:
	$(qemu) $(qemu_args)

asm:
	$(OBJDUMP) -d $(kernel) | less

clean:
	cargo clean
