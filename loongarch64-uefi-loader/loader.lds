OUTPUT_FORMAT("elf64-loongarch")
OUTPUT_ARCH(loongarch)
ENTRY(efi_main)

SECTIONS
{
  . = 0x1000;

  .text ALIGN(4096) : {
    *(.text.efi_main)
    *(.text .text.*)
  }

  .rodata ALIGN(4096) : {
    *(.srodata .srodata.*)
    *(.rodata .rodata.*)
  }

  .data ALIGN(4096) : {
    *(.sdata .sdata.*)
    *(.data .data.*)
  }

  .bss ALIGN(4096) (NOLOAD) : {
    __bss_start = .;
    *(.sbss .sbss.*)
    *(.bss .bss.*)
    *(COMMON)
    __bss_end = .;
  }

  .reloc ALIGN(4096) : {
    LONG(0);
    LONG(12);
    SHORT(0);
    SHORT(0);
  }

  /DISCARD/ : {
    *(.comment)
    *(.note*)
    *(.eh_frame*)
    *(.interp)
    *(.dyn*)
    *(.gnu*)
  }
}
