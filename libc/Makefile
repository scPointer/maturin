MUSL_LIB?=/opt/riscv/musl-riscv-toolchain-8.2.0-1/riscv64-linux-musl/lib
PREFIX?=riscv64-linux-musl-
CC=$(PREFIX)gcc
OBJCOPY=$(PREFIX)objcopy

STATIC_TESTCASES = $(shell cat static_testcases.txt)
OUTPUT_TESTCASES = $(addprefix build/,  $(subst -,_,$(notdir $(basename $(STATIC_TESTCASES)))))

CFLAGS += -pipe -std=c99 -D_POSIX_C_SOURCE=200809L -Wall -Wno-unused-function -Wno-missing-braces -Wno-unused -Wno-overflow
CFLAGS += -Wno-unknown-pragmas -fno-builtin -frounding-math
CFLAGS += -Werror=implicit-function-declaration -Werror=implicit-int -Werror=pointer-sign -Werror=pointer-arith
CFLAGS += -Os -s
CFLAGS += -Isrc/common/
LDLIBS += -Os -s -lpthread -lm -lrt

# 支持静态测例
CFLAGS += -static

COMMON_SRC_FILES = fdfill.c memfill.c mtest.c path.c print.c rand.c setrlim.c utf8.c vmfill.c
COMMON_SRCS = $(addprefix src/common/,  $(COMMON_SRC_FILES))


DIR ?= ./src/
TARGET_NAME ?= argv

all: build_static

build: $(DIR)$(TARGET_NAME).out

debug: $(DIR)$(TARGET_NAME).dbg

asm: $(DIR)$(TARGET_NAME).S

clean:
	find src -name '*.out' | xargs -t rm

%.out : %.c
	$(CC) $^ $(COMMON_SRCS) $(CFLAGS) -o $@
	mkdir -p build
	cp $@ $(addprefix build/,  $(subst -,_,$(notdir $(basename $@))))

src/functional/tls_align.out: src/functional/tls_align.c src/functional/tls_align_dso.c
	$(CC) $^ $(COMMON_SRCS) $(CFLAGS) -o $@	

%.dbg : %.c
	$(CC) $^ $(COMMON_SRCS) $(CFLAGS) -g -o $@ 

%.S : %.out
	$(OBJDUMP) -ld $^ > $@ 

test:
	@echo $(STATIC_TESTCASES)

build_static: $(STATIC_TESTCASES)

str_array_in_rust:
	ls build | xargs -i echo \"{}\",
	

.PHONY: all build debug asm clean test build_static
#$(CC) $(CFLAGS) -c $^ -o $@
#chmod +x $@