# ostool LoongArch64 UEFI Loader

This is the native LoongArch64 UEFI loader path for boards where DHCP HTTP Boot
URL delivery is not controllable.

It intentionally does not use iPXE. The firmware starts:

```text
EFI/BOOT/BOOTLOONGARCH64.EFI
```

The loader is implemented in Rust (`src/main.rs` and the modules under `src/`).
There is no C loader path in this directory.

The Rust loader brings up serial logging, probes the firmware networking
protocols, downloads `manifest.json` and `kernel.bin` over HTTPS, places the
kernel at `kernel_load_addr`, prepares framebuffer boot info, and jumps to
`entry_point` when `ENABLE_BOOT_JUMP=1`.

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
  MANIFEST_URL=https://10.3.10.229:3443/boot/boards/loongchip-httpboot-smoke/current/manifest.json
```

Enable the final jump only after download and memory-map observations are stable:

```bash
make -C loongarch64-uefi-loader ENABLE_BOOT_JUMP=1
```

Output:

```text
target/loongarch64-uefi-loader/BOOTLOONGARCH64.EFI
```
