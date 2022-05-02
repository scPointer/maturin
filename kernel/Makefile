ARCH ?= riscv64
MODE ?= debug
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

build_args := --target $(target)
ifeq ($(MODE), release)
build_args += --release
endif

qemu_args := -nographic -smp $(SMP)
ifeq ($(ARCH), riscv64)
qemu_args += \
	-machine virt \
	-bios default \
	-device loader,file=$(kernel_img),addr=0x80200000
endif

.PHONY: build kernel run qemu asm clean

build: $(kernel_img)

kernel:
	@cd ../user && make build
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
