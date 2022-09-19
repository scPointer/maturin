ARCH ?= riscv64
MODE ?= release
PLATFORM ?= qemu
SMP ?= 1
MACHINE ?= virt
SBI ?= default
ONLINE ?= 1

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
build_path := target/$(target)/$(MODE)
kernel := $(build_path)/maturin
kernel_img := $(build_path)/maturin.img
testcases_img := ../fat.img

build_args := --target $(target)
ifeq ($(MODE), release)
build_args += --release
endif

ifndef ($(ONLINE))
build_args += --offline
endif

qemu_args := -nographic -smp $(SMP) -m 8G
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
	@cd ../fs-init && cargo run --release -- -b -s ../oscomp_testcases/$(DISK_DIR)/ -t ../oscomp_testcases/$(DISK_DIR)/ -o ../

gcc-img: testcases-img
	mkdir ../foo
	sudo mount ../fat.img ../foo
	-sudo cp -r -a ../oscomp_testcases/gcc/* ../foo/
	sudo umount ../fat.img
	rmdir ../foo

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
	cargo doc $(build_args) --no-deps
