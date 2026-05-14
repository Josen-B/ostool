# ostool LoongArch64 UEFI Loader

This is the native LoongArch64 UEFI loader path for boards where DHCP HTTP Boot
URL delivery is not controllable.

It intentionally does not use iPXE. The firmware starts:

```text
EFI/BOOT/BOOTLOONGARCH64.EFI
```

The loader is implemented in Rust (`src/main.rs`). The previous C version is
kept as `loader.c` for reference while the board bring-up is still moving.

The Rust loader then uses UEFI HTTP Protocol to download `manifest.json`,
downloads `kernel.bin` to `kernel_load_addr`, prints the entry plan, and keeps
the final jump behind `ENABLE_BOOT_JUMP=0` until the board-side observations are
stable. It currently matches the C bring-up path, including the TLS
Configuration probe and embedded temporary CA certificate used during HTTPS
validation.

Build:

```bash
make -C loongarch64-uefi-loader
```

The build uses Rust target `loongarch64-unknown-none-softfloat`, links with the
same `loader.lds` layout that was validated by the firmware, then converts the
ELF to `pei-loongarch64` via `loongarch64-linux-gnu-objcopy`.

Override the manifest URL:

```bash
make -C loongarch64-uefi-loader \
  MANIFEST_URL=http://10.3.10.229:2999/boot/boards/loongchip-httpboot-smoke/current/manifest.json
```

Enable the final jump only after download and memory-map observations are stable:

```bash
make -C loongarch64-uefi-loader ENABLE_BOOT_JUMP=1
```

Output:

```text
target/loongarch64-uefi-loader/BOOTLOONGARCH64.EFI
```
