MUSL_LIB?=/opt/riscv/musl-riscv-toolchain-8.2.0-1/riscv64-linux-musl/lib
PREFIX?=riscv64-linux-musl-
CC=$(PREFIX)gcc
OBJCOPY=$(PREFIX)objcopy

CFLAGS += -pipe -std=c99 -D_POSIX_C_SOURCE=200809L -Wall -Wno-unused-function -Wno-missing-braces -Wno-unused -Wno-overflow
CFLAGS += -Wno-unknown-pragmas -fno-builtin -frounding-math
CFLAGS += -Werror=implicit-function-declaration -Werror=implicit-int -Werror=pointer-sign -Werror=pointer-arith
CFLAGS += -Os -s
CFLAGS += -Isrc/common/
LDLIBS += -Os -s -lpthread -lm -lrt

CFLAGS += -static


DIR=./src/
TARGET_NAME=argv

build: $(DIR)$(TARGET_NAME).out

clean:
	rm $(DIR)$(TARGET_NAME).out

%.out : %.c
	$(CC) $^ ./src/common/print.c $(CFLAGS) -o $@ 

#$(CC) $(CFLAGS) -c $^ -o $@
#chmod +x $@