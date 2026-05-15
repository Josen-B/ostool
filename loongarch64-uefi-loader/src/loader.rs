use core::ffi::c_void;
use core::ptr::{null_mut, read_volatile};

const OSTOOL_MANIFEST_URL: &str = env!("OSTOOL_MANIFEST_URL");
const OSTOOL_ENABLE_BOOT_JUMP: bool = cfg!(ostool_enable_boot_jump);
const OSTOOL_LOONGARCH64_LOADER_BUILD_ID: &str = "start43-real-boot-jump";

include!("abi.rs");
include!("console.rs");
include!("uefi.rs");
include!("tls.rs");
include!("tcp4.rs");
include!("http.rs");
include!("manifest.rs");
include!("boot.rs");
include!("flow.rs");
