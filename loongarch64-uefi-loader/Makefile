CROSS_COMPILE ?= loongarch64-linux-gnu-
LD := $(CROSS_COMPILE)ld
OBJCOPY := $(CROSS_COMPILE)objcopy
RUSTC ?= rustc
RUST_BOOT_JUMP_CFG :=
ifeq ($(ENABLE_BOOT_JUMP),1)
RUST_BOOT_JUMP_CFG := --cfg ostool_enable_boot_jump
endif

OUT_DIR ?= ../target/loongarch64-uefi-loader
TARGET := $(OUT_DIR)/BOOTLOONGARCH64.EFI
ELF := $(OUT_DIR)/ostool-loongarch64-loader.elf

MANIFEST_URL ?= https://10.3.10.229:3443/boot/boards/loongchip-httpboot-smoke/current/manifest.json
ENABLE_BOOT_JUMP ?= 0

LDFLAGS := -nostdlib -znocombreloc -T loader.lds

.PHONY: all clean

all: $(TARGET)

$(OUT_DIR):
	mkdir -p $@

RUST_SRCS := $(wildcard src/*.rs)

$(ELF): $(RUST_SRCS) loader.lds | $(OUT_DIR)
	OSTOOL_MANIFEST_URL="$(MANIFEST_URL)" \
	OSTOOL_ENABLE_BOOT_JUMP="$(ENABLE_BOOT_JUMP)" \
	$(RUSTC) --edition=2024 \
		--target loongarch64-unknown-none-softfloat \
		-C panic=abort \
		-C opt-level=s \
		-C relocation-model=static \
		-C code-model=medium \
		-C linker=$(LD) \
		-C link-arg=-nostdlib \
		-C link-arg=-znocombreloc \
		-C link-arg=-Tloader.lds \
		$(RUST_BOOT_JUMP_CFG) \
		-o $@ src/main.rs

$(TARGET): $(ELF)
	$(OBJCOPY) -O pei-loongarch64 \
		--subsystem=10 \
		-j .text -j .rodata -j .data -j .bss -j .reloc \
		$< $@

clean:
	rm -rf $(OUT_DIR)
