ARCH ?= riscv64
MODE ?= release
PLATFORM ?= qemu
SMP ?= 1
MACHINE ?= virt
SBI ?= default
ONLINE ?= 1
SIFIVE ?= y

OBJDUMP ?= rust-objdump
OBJCOPY ?= rust-objcopy
GDB ?= riscv64-unknown-elf-gdb

DISK_DIR ?= judge
# BOOTLOADER := ../bin/fw_jump.bin
export PLATFORM

ifeq ($(ARCH), riscv64)
target := riscv64imac-unknown-none-elf
#target := riscv64.json
else
$(error Unsupported architecture $(ARCH))
endif

qemu := qemu-system-$(ARCH)
cur_dir := $(shell pwd)
build_path := $(cur_dir)/target/$(target)/$(MODE)
kernel := $(build_path)/maturin
kernel_img := $(build_path)/maturin.img
disk_img_from := $(cur_dir)/../testcases/$(DISK_DIR)/
testcases_img := $(cur_dir)/fat.img

build_args := --target $(target)
ifeq ($(MODE), release)
build_args += --release
endif

ifneq ($(ONLINE), 1)
build_args += --offline
endif

ifeq ($(SIFIVE), y)
build_args += --features sifive
endif

qemu_args := -nographic -smp $(SMP) -m 1G
ifeq ($(ARCH), riscv64)
qemu_args += \
	-machine $(MACHINE) \
	-kernel $(kernel_img)
endif

ifeq ($(SBI), rustsbi)
qemu_args += -bios rustsbi-qemu.bin
else
qemu_args += -bios default
endif

.PHONY: build testcases-img kernel run qemu asm clean gdb-runner gdb-listener doc

#build: $(kernel_img) easy-fs-img
build: $(kernel_img)
	cp $(kernel_img) ../os.bin

testcases-img:
	@rm -f $(testcases_img)
	@cd ../modules/fs-init && cargo run --release -- -b -s $(disk_img_from) -t $(disk_img_from) -o $(testcases_img)

gcc-img: testcases-img
	mkdir ../foo
	sudo mount $(testcases_img) ../foo
	cd ../testcases/gcc && tar -zxvf ../riscv64-linux-musl-native.tgz
	-sudo cp -r -a ../testcases/gcc/* ../foo/
	sudo umount $(testcases_img)
	rmdir ../foo
	cd ../testcases/gcc && rm -rf ./riscv64-linux-musl-native

fu740: $(kernel_img)
	gzip -9 -cvf $(kernel_img) > ../os.bin.gz
	mkimage -f maturin-fu740.its ../os-fu740.itb
	
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
	cargo doc $(build_args) --no-deps --document-private-items
