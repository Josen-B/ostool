#![no_std]
#![no_main]

use core::ffi::c_void;
use core::panic::PanicInfo;
use core::ptr::{null_mut, read_volatile};

const OSTOOL_MANIFEST_URL: &str = env!("OSTOOL_MANIFEST_URL");
const OSTOOL_ENABLE_BOOT_JUMP: bool = cfg!(ostool_enable_boot_jump);

type EfiPhysicalAddress = u64;
type EfiVirtualAddress = u64;
type EfiHandle = *mut c_void;
type EfiEvent = *mut c_void;
type EfiTpl = usize;
type EfiMemoryType = u32;
type EfiAllocateType = u32;
type EfiLocateSearchType = u32;

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
struct EfiStatus(u64);

const EFI_SUCCESS: EfiStatus = EfiStatus(0);
const EFI_ERROR_BIT: u64 = 1 << 63;
const EFI_UNSUPPORTED: EfiStatus = EfiStatus(EFI_ERROR_BIT | 3);
const EFI_BUFFER_TOO_SMALL: EfiStatus = EfiStatus(EFI_ERROR_BIT | 5);
const EFI_NOT_READY: EfiStatus = EfiStatus(EFI_ERROR_BIT | 6);
const EFI_DEVICE_ERROR: EfiStatus = EfiStatus(EFI_ERROR_BIT | 7);
const EFI_NOT_FOUND: EfiStatus = EfiStatus(EFI_ERROR_BIT | 14);

const EFI_ALLOCATE_ADDRESS: EfiAllocateType = 0;
const EFI_LOADER_DATA: EfiMemoryType = 2;
const EFI_LOCATE_BY_PROTOCOL: EfiLocateSearchType = 2;
const EVT_NOTIFY_SIGNAL: u32 = 0x0000_0200;
const TPL_CALLBACK: EfiTpl = 8;
const EFI_PAGE_SIZE: usize = 4096;

const HTTP_VERSION_11: u32 = 1;
const HTTP_METHOD_GET: u32 = 0;
const HTTP_STATUS_200_OK: u32 = 3;

const MANIFEST_MAX: usize = 4096;
const URL16_MAX: usize = 1024;
const KERNEL_CHUNK: usize = 16 * 1024;
const MAX_KERNEL_SIZE: u64 = 256 * 1024 * 1024;
const MEMORY_MAP_MAX: usize = 64 * 1024;
const HTTP_POLL_LIMIT: usize = 1_000_000;

const EFI_TLS_CONFIG_DATA_TYPE_CA_CERTIFICATE: u32 = 2;

#[repr(C)]
#[derive(Clone, Copy)]
struct EfiGuid {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

#[repr(C)]
struct EfiTableHeader {
    signature: u64,
    revision: u32,
    header_size: u32,
    crc32: u32,
    reserved: u32,
}

#[repr(C)]
struct EfiSimpleTextOutputProtocol {
    reset: usize,
    output_string: extern "C" fn(*mut EfiSimpleTextOutputProtocol, *const u16) -> EfiStatus,
}

#[repr(C)]
struct EfiMemoryDescriptor {
    memory_type: u32,
    physical_start: EfiPhysicalAddress,
    virtual_start: EfiVirtualAddress,
    number_of_pages: u64,
    attribute: u64,
}

#[repr(C)]
struct EfiBootServices {
    hdr: EfiTableHeader,
    raise_tpl: usize,
    restore_tpl: usize,
    allocate_pages:
        extern "C" fn(EfiAllocateType, EfiMemoryType, usize, *mut EfiPhysicalAddress) -> EfiStatus,
    free_pages: extern "C" fn(EfiPhysicalAddress, usize) -> EfiStatus,
    get_memory_map: extern "C" fn(
        *mut usize,
        *mut EfiMemoryDescriptor,
        *mut usize,
        *mut usize,
        *mut u32,
    ) -> EfiStatus,
    allocate_pool: extern "C" fn(EfiMemoryType, usize, *mut *mut c_void) -> EfiStatus,
    free_pool: extern "C" fn(*mut c_void) -> EfiStatus,
    create_event: extern "C" fn(
        u32,
        EfiTpl,
        Option<extern "C" fn(EfiEvent, *mut c_void)>,
        *mut c_void,
        *mut EfiEvent,
    ) -> EfiStatus,
    set_timer: usize,
    wait_for_event: usize,
    signal_event: usize,
    close_event: extern "C" fn(EfiEvent) -> EfiStatus,
    check_event: usize,
    install_protocol_interface: usize,
    reinstall_protocol_interface: usize,
    uninstall_protocol_interface: usize,
    handle_protocol: extern "C" fn(EfiHandle, *const EfiGuid, *mut *mut c_void) -> EfiStatus,
    reserved: usize,
    register_protocol_notify: usize,
    locate_handle: usize,
    locate_device_path: usize,
    install_configuration_table: usize,
    load_image: usize,
    start_image: usize,
    exit: usize,
    unload_image: usize,
    exit_boot_services: extern "C" fn(EfiHandle, usize) -> EfiStatus,
    get_next_monotonic_count: usize,
    stall: Option<extern "C" fn(usize) -> EfiStatus>,
    set_watchdog_timer: usize,
    connect_controller: usize,
    disconnect_controller: usize,
    open_protocol: usize,
    close_protocol: usize,
    open_protocol_information: usize,
    protocols_per_handle: usize,
    locate_handle_buffer: extern "C" fn(
        EfiLocateSearchType,
        *const EfiGuid,
        *mut c_void,
        *mut usize,
        *mut *mut EfiHandle,
    ) -> EfiStatus,
}

#[repr(C)]
struct EfiSystemTable {
    hdr: EfiTableHeader,
    firmware_vendor: *mut u16,
    firmware_revision: u32,
    console_in_handle: EfiHandle,
    con_in: *mut c_void,
    console_out_handle: EfiHandle,
    con_out: *mut EfiSimpleTextOutputProtocol,
    standard_error_handle: EfiHandle,
    std_err: *mut EfiSimpleTextOutputProtocol,
    runtime_services: *mut c_void,
    boot_services: *mut EfiBootServices,
}

#[repr(C)]
struct EfiServiceBindingProtocol {
    create_child: extern "C" fn(*mut EfiServiceBindingProtocol, *mut EfiHandle) -> EfiStatus,
    destroy_child: extern "C" fn(*mut EfiServiceBindingProtocol, EfiHandle) -> EfiStatus,
}

#[repr(C)]
union EfiHttpConfigAccessPoint {
    ipv4_node: *mut EfiHttpv4AccessPoint,
    ipv6_node: *mut c_void,
}

#[repr(C)]
struct EfiHttpv4AccessPoint {
    use_default_address: u8,
    local_address: [u8; 4],
    local_subnet: [u8; 4],
    local_port: u16,
}

#[repr(C)]
struct EfiHttpConfigData {
    http_version: u32,
    timeout_millisec: u32,
    local_address_is_ipv6: u8,
    padding: [u8; 7],
    access_point: EfiHttpConfigAccessPoint,
}

#[repr(C)]
struct EfiHttpRequestData {
    method: u32,
    url: *mut u16,
}

#[repr(C)]
struct EfiHttpResponseData {
    status_code: u32,
}

#[repr(C)]
union EfiHttpMessageData {
    request: *mut EfiHttpRequestData,
    response: *mut EfiHttpResponseData,
}

#[repr(C)]
struct EfiHttpHeader {
    field_name: *mut u8,
    field_value: *mut u8,
}

#[repr(C)]
struct EfiHttpMessage {
    data: EfiHttpMessageData,
    header_count: usize,
    headers: *mut EfiHttpHeader,
    body_length: usize,
    body: *mut c_void,
}

#[repr(C)]
struct EfiHttpToken {
    event: EfiEvent,
    status: EfiStatus,
    message: *mut EfiHttpMessage,
}

#[repr(C)]
struct EfiHttpProtocol {
    get_mode_data: extern "C" fn(*mut EfiHttpProtocol, *mut EfiHttpConfigData) -> EfiStatus,
    configure: extern "C" fn(*mut EfiHttpProtocol, *mut EfiHttpConfigData) -> EfiStatus,
    request: extern "C" fn(*mut EfiHttpProtocol, *mut EfiHttpToken) -> EfiStatus,
    cancel: usize,
    response: extern "C" fn(*mut EfiHttpProtocol, *mut EfiHttpToken) -> EfiStatus,
    poll: extern "C" fn(*mut EfiHttpProtocol) -> EfiStatus,
}

#[repr(C)]
struct EfiTlsConfigurationProtocol {
    set_data: extern "C" fn(*mut EfiTlsConfigurationProtocol, u32, *mut c_void, usize) -> EfiStatus,
    get_data:
        extern "C" fn(*mut EfiTlsConfigurationProtocol, u32, *mut c_void, *mut usize) -> EfiStatus,
}

#[derive(Clone, Copy)]
struct Manifest {
    kernel_url: [u8; 1024],
    kernel_url_len: usize,
    kernel_size: u64,
    kernel_load_addr: u64,
    entry_point: u64,
    arch: [u8; 32],
    arch_len: usize,
}

const EFI_HTTP_SERVICE_BINDING_PROTOCOL_GUID: EfiGuid = EfiGuid {
    data1: 0xbdc8e6af,
    data2: 0xd9bc,
    data3: 0x4379,
    data4: [0xa7, 0x2a, 0xe0, 0xc4, 0xe7, 0x5d, 0xae, 0x1c],
};

const EFI_HTTP_PROTOCOL_GUID: EfiGuid = EfiGuid {
    data1: 0x7a59b29b,
    data2: 0x910b,
    data3: 0x4171,
    data4: [0x82, 0x42, 0xa8, 0x5a, 0x0d, 0xf2, 0x5b, 0x5b],
};

const EFI_TLS_SERVICE_BINDING_PROTOCOL_GUID: EfiGuid = EfiGuid {
    data1: 0x952cb795,
    data2: 0xff36,
    data3: 0x48cf,
    data4: [0xa2, 0x49, 0x4d, 0xf4, 0x86, 0xd6, 0xab, 0x8d],
};

const EFI_TLS_CONFIGURATION_PROTOCOL_GUID: EfiGuid = EfiGuid {
    data1: 0x1682fe44,
    data2: 0xbd7a,
    data3: 0x4407,
    data4: [0xb7, 0xc7, 0xdc, 0xa3, 0x7c, 0xa3, 0x92, 0x2d],
};

const EFI_TCP4_SERVICE_BINDING_PROTOCOL_GUID: EfiGuid = EfiGuid {
    data1: 0x00720665,
    data2: 0x67eb,
    data3: 0x4a99,
    data4: [0xba, 0xf7, 0xd3, 0xc3, 0x3a, 0x1c, 0x7c, 0xc9],
};

static HTTPS_CA_PEM: &[u8] = b"-----BEGIN CERTIFICATE-----\n\
MIIDHjCCAgagAwIBAgIUBJubHQIousJm3ZT9sNPYq0u1AhwwDQYJKoZIhvcNAQEL\n\
BQAwFjEUMBIGA1UEAwwLMTAuMy4xMC4yMjkwHhcNMjYwNTEzMDkxMTIwWhcNMjYw\n\
NTIwMDkxMTIwWjAWMRQwEgYDVQQDDAsxMC4zLjEwLjIyOTCCASIwDQYJKoZIhvcN\n\
AQEBBQADggEPADCCAQoCggEBALb3klUcff8fXYIcsgeQr1gs2rnwbOl/4Unwtulx\n\
wG1K8joXYwWT4NP4XSOJy8aVuLk0FSd8VB29l6gjduSYzdC1CE9i3bzJnu4E96X/\n\
EqRWP6QPkQJUizpH3qwxK1sDNJTmoAdq48v3cLgyyDdxzU/iVlWM51izk4njFMzZ\n\
4PCLFfznANnj8o5diDCQ96uyKxmaXArIeDAAwcTSlJYc7QHWg6WEg+FQcn3TaMKJ\n\
8rNELrKMrygc71ZdF9r6anud4YMouse6wJmEzGEVSCQ/y3dxd8gr1Ixq3DV9Yj9J\n\
fZ57GWjWLTi1CWyMAAod8b6xr+o2yHRS2mGIfTEskTHTc+8CAwEAAaNkMGIwHQYD\n\
VR0OBBYEFIimL7eY9PEocEUee1gz/YxTalmbMB8GA1UdIwQYMBaAFIimL7eY9PEo\n\
cEUee1gz/YxTalmbMA8GA1UdEwEB/wQFMAMBAf8wDwYDVR0RBAgwBocECgMK5TAN\n\
BgkqhkiG9w0BAQsFAAOCAQEAf1TkdDogQAYDgSUdraZ6WtOrD7MrLH69DZIcMrVf\n\
GymOgar70uD9s1MEAwAsCgfqN8+kRcR/viWY8e86AzYVralqiLVs9tpR+vrnFejd\n\
f9KLftc3owFwmiMLR5szwZMENOz2F+TJ8fNZBTXaJuITxrcIwuBym0FqL1pkN4hL\n\
ikfU5paqfDst5LA/Wu/56XPtP8tFGh498jNsKlAumlQgaX0w+xxGiaGf1WkvTOP8\n\
bVwyUYVeTIG2utpOKra0gkg42qcPdvRzZsT9REzlp2cxyBx5fkSmS0kXtqg4fT69\n\
9/dZPxTWismXdZ4HN74kKak5tAB9CwKXvwaLRqOmXEg4hw==\n\
-----END CERTIFICATE-----\n";

static mut CONSOLE: *mut EfiSimpleTextOutputProtocol = null_mut();
static mut MEMORY_MAP: [u8; MEMORY_MAP_MAX] = [0; MEMORY_MAP_MAX];

impl EfiStatus {
    fn is_error(self) -> bool {
        (self.0 & EFI_ERROR_BIT) != 0
    }
}

fn console() -> *mut EfiSimpleTextOutputProtocol {
    unsafe { CONSOLE }
}

fn write_ascii(s: &str) {
    write_bytes(s.as_bytes());
}

fn write_bytes(s: &[u8]) {
    let out = console();
    if out.is_null() {
        return;
    }
    let mut pos = 0;
    while pos < s.len() {
        let mut buf = [0u16; 256];
        let mut n = 0;
        while pos + n < s.len() && n + 1 < buf.len() {
            buf[n] = s[pos + n] as u16;
            n += 1;
        }
        buf[n] = 0;
        unsafe {
            ((*out).output_string)(out, buf.as_ptr());
        }
        pos += n;
    }
}

fn write_hex64(value: u64) {
    let mut out = [0u8; 16];
    let mut i = 0;
    while i < 16 {
        let digit = ((value >> ((15 - i) * 4)) & 0xf) as u8;
        out[i] = if digit < 10 {
            b'0' + digit
        } else {
            b'a' + digit - 10
        };
        i += 1;
    }
    write_bytes(&out);
}

fn write_dec(mut value: u64) {
    let mut out = [0u8; 32];
    let mut pos = out.len();
    if value == 0 {
        write_ascii("0");
        return;
    }
    while value > 0 && pos > 0 {
        pos -= 1;
        out[pos] = b'0' + (value % 10) as u8;
        value /= 10;
    }
    write_bytes(&out[pos..]);
}

fn write_status(label: &str, status: EfiStatus) {
    write_ascii(label);
    write_ascii("0x");
    write_hex64(status.0);
    write_ascii("\r\n");
}

fn write_utf16_url(url: &[u8], out: &mut [u16]) -> Result<*mut u16, EfiStatus> {
    if url.len() + 1 > out.len() {
        return Err(EFI_BUFFER_TOO_SMALL);
    }
    let mut i = 0;
    while i < url.len() {
        out[i] = url[i] as u16;
        i += 1;
    }
    out[i] = 0;
    Ok(out.as_mut_ptr())
}

extern "C" fn noop_event(_event: EfiEvent, _context: *mut c_void) {}

fn poll_http(http: *mut EfiHttpProtocol, token: *const EfiHttpToken) -> EfiStatus {
    let mut i = 0;
    while i < HTTP_POLL_LIMIT {
        let status = unsafe { read_volatile(core::ptr::addr_of!((*token).status)) };
        if status != EFI_NOT_READY {
            return status;
        }
        unsafe {
            ((*http).poll)(http);
        }
        i += 1;
    }
    unsafe { read_volatile(core::ptr::addr_of!((*token).status)) }
}

fn warm_up_http(bs: *mut EfiBootServices, http: *mut EfiHttpProtocol) {
    let mut last_poll = EFI_NOT_READY;
    let mut i = 0;
    while i < 20 {
        unsafe {
            if let Some(stall) = (*bs).stall {
                stall(100_000);
            }
            last_poll = ((*http).poll)(http);
        }
        i += 1;
    }
    write_status("http_post_configure_poll_status: ", last_poll);
}

fn locate_protocol_handles(
    bs: *mut EfiBootServices,
    guid: &EfiGuid,
    count: &mut usize,
    handles: &mut *mut EfiHandle,
) -> EfiStatus {
    *count = 0;
    *handles = null_mut();
    unsafe {
        ((*bs).locate_handle_buffer)(
            EFI_LOCATE_BY_PROTOCOL,
            guid as *const EfiGuid,
            null_mut(),
            count,
            handles,
        )
    }
}

fn print_protocol_handle_count(bs: *mut EfiBootServices, label: &str, guid: &EfiGuid) {
    let mut count = 0usize;
    let mut handles = null_mut();
    let status = locate_protocol_handles(bs, guid, &mut count, &mut handles);
    write_ascii(label);
    write_ascii("_status: ");
    write_ascii("0x");
    write_hex64(status.0);
    write_ascii("\r\n");
    write_ascii(label);
    write_ascii("_handle_count: ");
    write_dec(count as u64);
    write_ascii("\r\n");
    if !status.is_error() && !handles.is_null() {
        unsafe {
            ((*bs).free_pool)(handles as *mut c_void);
        }
    }
}

fn open_protocol<T>(
    bs: *mut EfiBootServices,
    handle: EfiHandle,
    guid: &EfiGuid,
) -> Result<*mut T, EfiStatus> {
    let mut interface = null_mut();
    let status = unsafe { ((*bs).handle_protocol)(handle, guid as *const EfiGuid, &mut interface) };
    if status.is_error() || interface.is_null() {
        return Err(if status.is_error() {
            status
        } else {
            EFI_UNSUPPORTED
        });
    }
    Ok(interface as *mut T)
}

fn b64_value(ch: u8) -> Option<u8> {
    match ch {
        b'A'..=b'Z' => Some(ch - b'A'),
        b'a'..=b'z' => Some(ch - b'a' + 26),
        b'0'..=b'9' => Some(ch - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

fn pem_to_der(pem: &[u8], out: &mut [u8]) -> Result<usize, EfiStatus> {
    let mut in_body = false;
    let mut accum = 0u32;
    let mut bits = 0u32;
    let mut len = 0usize;
    let mut i = 0usize;
    while i < pem.len() {
        let ch = pem[i];
        if !in_body {
            if ch == b'\n' {
                in_body = true;
            }
            i += 1;
            continue;
        }
        if ch == b'-' {
            break;
        }
        if ch == b'=' {
            break;
        }
        if let Some(value) = b64_value(ch) {
            accum = (accum << 6) | value as u32;
            bits += 6;
            while bits >= 8 {
                bits -= 8;
                if len >= out.len() {
                    return Err(EFI_BUFFER_TOO_SMALL);
                }
                out[len] = ((accum >> bits) & 0xff) as u8;
                len += 1;
            }
        }
        i += 1;
    }
    Ok(len)
}

fn set_tls_ca_variants(tls_config: *mut EfiTlsConfigurationProtocol, label: &str) {
    let set_pem_status = unsafe {
        ((*tls_config).set_data)(
            tls_config,
            EFI_TLS_CONFIG_DATA_TYPE_CA_CERTIFICATE,
            HTTPS_CA_PEM.as_ptr() as *mut c_void,
            HTTPS_CA_PEM.len(),
        )
    };
    if label == "tls_config" {
        write_status("tls_config_set_ca_status: ", set_pem_status);
    } else {
        write_ascii(label);
        write_ascii("_set_ca_pem_status: 0x");
        write_hex64(set_pem_status.0);
        write_ascii("\r\n");
    }

    let mut der = [0u8; 1536];
    match pem_to_der(HTTPS_CA_PEM, &mut der) {
        Ok(der_len) => {
            write_ascii(label);
            write_ascii("_ca_der_size: ");
            write_dec(der_len as u64);
            write_ascii("\r\n");
            let set_der_status = unsafe {
                ((*tls_config).set_data)(
                    tls_config,
                    EFI_TLS_CONFIG_DATA_TYPE_CA_CERTIFICATE,
                    der.as_mut_ptr() as *mut c_void,
                    der_len,
                )
            };
            write_ascii(label);
            write_ascii("_set_ca_der_status: 0x");
            write_hex64(set_der_status.0);
            write_ascii("\r\n");
        }
        Err(status) => {
            write_ascii(label);
            write_ascii("_ca_der_decode_status: 0x");
            write_hex64(status.0);
            write_ascii("\r\n");
        }
    }
}

fn configure_tls_ca(bs: *mut EfiBootServices) {
    let mut service_count = 0usize;
    let mut service_handles = null_mut();
    let status = locate_protocol_handles(
        bs,
        &EFI_TLS_SERVICE_BINDING_PROTOCOL_GUID,
        &mut service_count,
        &mut service_handles,
    );
    write_status("tls_config_service_status: ", status);
    write_ascii("tls_config_service_handle_count: ");
    write_dec(service_count as u64);
    write_ascii("\r\n");
    if status.is_error() || service_count == 0 || service_handles.is_null() {
        return;
    }

    let service_handle = unsafe { *service_handles };
    let binding = match open_protocol::<EfiServiceBindingProtocol>(
        bs,
        service_handle,
        &EFI_TLS_SERVICE_BINDING_PROTOCOL_GUID,
    ) {
        Ok(binding) => {
            write_status("tls_config_binding_open_status: ", EFI_SUCCESS);
            binding
        }
        Err(status) => {
            write_status("tls_config_binding_open_status: ", status);
            unsafe {
                ((*bs).free_pool)(service_handles as *mut c_void);
            }
            return;
        }
    };

    let mut child = null_mut();
    let status = unsafe { ((*binding).create_child)(binding, &mut child) };
    write_status("tls_config_create_child_status: ", status);
    if status.is_error() || child.is_null() {
        unsafe {
            ((*bs).free_pool)(service_handles as *mut c_void);
        }
        return;
    }

    match open_protocol::<EfiTlsConfigurationProtocol>(
        bs,
        child,
        &EFI_TLS_CONFIGURATION_PROTOCOL_GUID,
    ) {
        Ok(tls_config) => {
            write_status("tls_config_protocol_status: ", EFI_SUCCESS);
            let mut ca_size = 0usize;
            let get_status = unsafe {
                ((*tls_config).get_data)(
                    tls_config,
                    EFI_TLS_CONFIG_DATA_TYPE_CA_CERTIFICATE,
                    null_mut(),
                    &mut ca_size,
                )
            };
            write_status("tls_config_get_ca_status: ", get_status);
            write_ascii("tls_config_get_ca_size: ");
            write_dec(ca_size as u64);
            write_ascii("\r\n");

            set_tls_ca_variants(tls_config, "tls_config");

            ca_size = 0;
            let get_after_status = unsafe {
                ((*tls_config).get_data)(
                    tls_config,
                    EFI_TLS_CONFIG_DATA_TYPE_CA_CERTIFICATE,
                    null_mut(),
                    &mut ca_size,
                )
            };
            write_status("tls_config_get_ca_after_status: ", get_after_status);
            write_ascii("tls_config_get_ca_after_size: ");
            write_dec(ca_size as u64);
            write_ascii("\r\n");
        }
        Err(status) => write_status("tls_config_protocol_status: ", status),
    }

    let destroy_status = unsafe { ((*binding).destroy_child)(binding, child) };
    write_status("tls_config_destroy_child_status: ", destroy_status);
    unsafe {
        ((*bs).free_pool)(service_handles as *mut c_void);
    }
}

fn configure_tls_ca_on_handle(bs: *mut EfiBootServices, handle: EfiHandle, label: &str) {
    match open_protocol::<EfiTlsConfigurationProtocol>(
        bs,
        handle,
        &EFI_TLS_CONFIGURATION_PROTOCOL_GUID,
    ) {
        Ok(tls_config) => {
            write_ascii(label);
            write_ascii("_protocol_status: 0x");
            write_hex64(EFI_SUCCESS.0);
            write_ascii("\r\n");

            let mut ca_size = 0usize;
            let get_status = unsafe {
                ((*tls_config).get_data)(
                    tls_config,
                    EFI_TLS_CONFIG_DATA_TYPE_CA_CERTIFICATE,
                    null_mut(),
                    &mut ca_size,
                )
            };
            write_ascii(label);
            write_ascii("_get_ca_status: 0x");
            write_hex64(get_status.0);
            write_ascii("\r\n");
            write_ascii(label);
            write_ascii("_get_ca_size: ");
            write_dec(ca_size as u64);
            write_ascii("\r\n");

            set_tls_ca_variants(tls_config, label);

            ca_size = 0;
            let get_after_status = unsafe {
                ((*tls_config).get_data)(
                    tls_config,
                    EFI_TLS_CONFIG_DATA_TYPE_CA_CERTIFICATE,
                    null_mut(),
                    &mut ca_size,
                )
            };
            write_ascii(label);
            write_ascii("_get_ca_after_status: 0x");
            write_hex64(get_after_status.0);
            write_ascii("\r\n");
            write_ascii(label);
            write_ascii("_get_ca_after_size: ");
            write_dec(ca_size as u64);
            write_ascii("\r\n");
        }
        Err(status) => {
            write_ascii(label);
            write_ascii("_protocol_status: 0x");
            write_hex64(status.0);
            write_ascii("\r\n");
        }
    }
}

fn configure_http(http: *mut EfiHttpProtocol) -> EfiStatus {
    let mut ipv4 = EfiHttpv4AccessPoint {
        use_default_address: 1,
        local_address: [0; 4],
        local_subnet: [0; 4],
        local_port: 0,
    };
    let mut config = EfiHttpConfigData {
        http_version: HTTP_VERSION_11,
        timeout_millisec: 0,
        local_address_is_ipv6: 0,
        padding: [0; 7],
        access_point: EfiHttpConfigAccessPoint {
            ipv4_node: &mut ipv4,
        },
    };
    unsafe { ((*http).configure)(http, &mut config) }
}

fn print_http_mode_data(http: *mut EfiHttpProtocol, label: &str) {
    let mut ipv4 = EfiHttpv4AccessPoint {
        use_default_address: 0,
        local_address: [0; 4],
        local_subnet: [0; 4],
        local_port: 0,
    };
    let mut config = EfiHttpConfigData {
        http_version: 0,
        timeout_millisec: 0,
        local_address_is_ipv6: 0,
        padding: [0; 7],
        access_point: EfiHttpConfigAccessPoint {
            ipv4_node: &mut ipv4,
        },
    };
    let status = unsafe { ((*http).get_mode_data)(http, &mut config) };
    write_ascii(label);
    write_ascii("_get_mode_data_status: 0x");
    write_hex64(status.0);
    write_ascii("\r\n");
    write_ascii(label);
    write_ascii("_mode_http_version: ");
    write_dec(config.http_version as u64);
    write_ascii("\r\n");
    write_ascii(label);
    write_ascii("_mode_timeout_ms: ");
    write_dec(config.timeout_millisec as u64);
    write_ascii("\r\n");
    write_ascii(label);
    write_ascii("_mode_ipv6: ");
    write_dec(config.local_address_is_ipv6 as u64);
    write_ascii("\r\n");
    write_ascii(label);
    write_ascii("_mode_ipv4_default: ");
    write_dec(ipv4.use_default_address as u64);
    write_ascii("\r\n");
    write_ascii(label);
    write_ascii("_mode_ipv4_local_port: ");
    write_dec(ipv4.local_port as u64);
    write_ascii("\r\n");
}

fn http_request_with_host(
    bs: *mut EfiBootServices,
    http: *mut EfiHttpProtocol,
    url: &[u8],
    label: &str,
    host: Option<&[u8]>,
) -> EfiStatus {
    let mut url16 = [0u16; URL16_MAX];
    let url16_ptr = match write_utf16_url(url, &mut url16) {
        Ok(ptr) => ptr,
        Err(status) => return status,
    };

    let mut event = null_mut();
    let status = unsafe {
        ((*bs).create_event)(
            EVT_NOTIFY_SIGNAL,
            TPL_CALLBACK,
            Some(noop_event),
            null_mut(),
            &mut event,
        )
    };
    write_ascii(label);
    write_ascii("_request_event_status: ");
    write_ascii("0x");
    write_hex64(status.0);
    write_ascii("\r\n");
    if status.is_error() {
        return status;
    }

    let mut request_data = EfiHttpRequestData {
        method: HTTP_METHOD_GET,
        url: url16_ptr,
    };
    let mut host_name = [b'H', b'o', b's', b't', 0];
    let mut host_value = [0u8; 96];
    let mut header = EfiHttpHeader {
        field_name: host_name.as_mut_ptr(),
        field_value: host_value.as_mut_ptr(),
    };
    let mut header_count = 0usize;
    let mut headers = null_mut();
    if let Some(host) = host {
        if host.len() + 1 <= host_value.len() {
            let mut i = 0usize;
            while i < host.len() {
                host_value[i] = host[i];
                i += 1;
            }
            host_value[i] = 0;
            header_count = 1;
            headers = &mut header;
        } else {
            unsafe {
                ((*bs).close_event)(event);
            }
            return EFI_BUFFER_TOO_SMALL;
        }
    }
    let mut message = EfiHttpMessage {
        data: EfiHttpMessageData {
            request: &mut request_data,
        },
        header_count,
        headers,
        body_length: 0,
        body: null_mut(),
    };
    let mut token = EfiHttpToken {
        event,
        status: EFI_NOT_READY,
        message: &mut message,
    };

    let submit_status = unsafe { ((*http).request)(http, &mut token) };
    write_ascii(label);
    write_ascii("_request_submit_status: ");
    write_ascii("0x");
    write_hex64(submit_status.0);
    write_ascii("\r\n");
    let mut final_status = submit_status;
    if !submit_status.is_error() {
        final_status = poll_http(http, &token);
    } else if submit_status == EFI_NOT_FOUND {
        write_ascii(label);
        write_ascii("_request_not_found_continue_response: yes\r\n");
        unsafe {
            ((*http).poll)(http);
        }
        final_status = EFI_SUCCESS;
    }
    write_ascii(label);
    write_ascii("_request_token_status: ");
    write_ascii("0x");
    write_hex64(token.status.0);
    write_ascii("\r\n");
    unsafe {
        ((*bs).close_event)(event);
    }
    final_status
}

fn http_request(
    bs: *mut EfiBootServices,
    http: *mut EfiHttpProtocol,
    url: &[u8],
    label: &str,
) -> EfiStatus {
    http_request_with_host(bs, http, url, label, None)
}

fn http_get_once_with_host(
    bs: *mut EfiBootServices,
    http: *mut EfiHttpProtocol,
    url: &[u8],
    label: &str,
    host: Option<&[u8]>,
    body: *mut u8,
    body_len: &mut usize,
    http_status: &mut u32,
) -> EfiStatus {
    let mut url16 = [0u16; URL16_MAX];
    let url16_ptr = match write_utf16_url(url, &mut url16) {
        Ok(ptr) => ptr,
        Err(status) => return status,
    };

    let mut request_event = null_mut();
    let status = unsafe {
        ((*bs).create_event)(
            EVT_NOTIFY_SIGNAL,
            TPL_CALLBACK,
            Some(noop_event),
            null_mut(),
            &mut request_event,
        )
    };
    write_ascii(label);
    write_ascii("_request_event_status: 0x");
    write_hex64(status.0);
    write_ascii("\r\n");
    if status.is_error() {
        return status;
    }

    let mut request_data = EfiHttpRequestData {
        method: HTTP_METHOD_GET,
        url: url16_ptr,
    };
    let mut host_name = [b'H', b'o', b's', b't', 0];
    let mut host_value = [0u8; 96];
    let mut header = EfiHttpHeader {
        field_name: host_name.as_mut_ptr(),
        field_value: host_value.as_mut_ptr(),
    };
    let mut header_count = 0usize;
    let mut headers = null_mut();
    if let Some(host) = host {
        if host.len() + 1 <= host_value.len() {
            let mut i = 0usize;
            while i < host.len() {
                host_value[i] = host[i];
                i += 1;
            }
            host_value[i] = 0;
            header_count = 1;
            headers = &mut header;
        } else {
            unsafe {
                ((*bs).close_event)(request_event);
            }
            return EFI_BUFFER_TOO_SMALL;
        }
    }

    let mut request_message = EfiHttpMessage {
        data: EfiHttpMessageData {
            request: &mut request_data,
        },
        header_count,
        headers,
        body_length: 0,
        body: null_mut(),
    };
    let mut request_token = EfiHttpToken {
        event: request_event,
        status: EFI_NOT_READY,
        message: &mut request_message,
    };

    let submit_status = unsafe { ((*http).request)(http, &mut request_token) };
    write_ascii(label);
    write_ascii("_request_submit_status: 0x");
    write_hex64(submit_status.0);
    write_ascii("\r\n");
    let mut request_status = submit_status;
    if !submit_status.is_error() {
        request_status = poll_http(http, &request_token);
    } else if submit_status == EFI_NOT_FOUND {
        write_ascii(label);
        write_ascii("_request_not_found_keep_context: yes\r\n");
        unsafe {
            ((*http).poll)(http);
        }
        request_status = EFI_SUCCESS;
    }
    write_ascii(label);
    write_ascii("_request_token_status: 0x");
    write_hex64(request_token.status.0);
    write_ascii("\r\n");
    write_prefixed_status(label, "_request_completion", request_status);
    if request_status.is_error() {
        unsafe {
            ((*bs).close_event)(request_event);
        }
        return request_status;
    }

    let mut response_event = null_mut();
    let status = unsafe {
        ((*bs).create_event)(
            EVT_NOTIFY_SIGNAL,
            TPL_CALLBACK,
            Some(noop_event),
            null_mut(),
            &mut response_event,
        )
    };
    write_ascii(label);
    write_ascii("_response_event_status: 0x");
    write_hex64(status.0);
    write_ascii("\r\n");
    if status.is_error() {
        unsafe {
            ((*bs).close_event)(request_event);
        }
        return status;
    }

    let mut response_data = EfiHttpResponseData { status_code: 0 };
    let mut response_message = EfiHttpMessage {
        data: EfiHttpMessageData {
            response: &mut response_data,
        },
        header_count: 0,
        headers: null_mut(),
        body_length: *body_len,
        body: body as *mut c_void,
    };
    let mut response_token = EfiHttpToken {
        event: response_event,
        status: EFI_NOT_READY,
        message: &mut response_message,
    };

    let mut response_status = unsafe { ((*http).response)(http, &mut response_token) };
    write_ascii(label);
    write_ascii("_response_submit_status: 0x");
    write_hex64(response_status.0);
    write_ascii("\r\n");
    if !response_status.is_error() {
        response_status = poll_http(http, &response_token);
    }
    write_ascii(label);
    write_ascii("_response_token_status: 0x");
    write_hex64(response_token.status.0);
    write_ascii("\r\n");
    *body_len = response_message.body_length;
    *http_status = response_data.status_code;
    if !response_message.headers.is_null() {
        unsafe {
            ((*bs).free_pool)(response_message.headers as *mut c_void);
        }
    }
    unsafe {
        ((*bs).close_event)(response_event);
        ((*bs).close_event)(request_event);
    }
    response_status
}

fn make_https_443_url(input: &[u8], out: &mut [u8]) -> Result<usize, EfiStatus> {
    const PREFIX: &[u8] = b"https://";
    const PORT: &[u8] = b":3443/";
    if input.len() <= PREFIX.len() || !starts_with(input, PREFIX) {
        return Err(EFI_UNSUPPORTED);
    }
    let mut i = PREFIX.len();
    while i + PORT.len() <= input.len() {
        if starts_with(&input[i..], PORT) {
            let mut n = 0usize;
            while n < i {
                if n >= out.len() {
                    return Err(EFI_BUFFER_TOO_SMALL);
                }
                out[n] = input[n];
                n += 1;
            }
            let path_start = i + PORT.len() - 1;
            let mut j = path_start;
            while j < input.len() {
                if n >= out.len() {
                    return Err(EFI_BUFFER_TOO_SMALL);
                }
                out[n] = input[j];
                n += 1;
                j += 1;
            }
            return Ok(n);
        }
        i += 1;
    }
    Err(EFI_UNSUPPORTED)
}

fn starts_with(haystack: &[u8], needle: &[u8]) -> bool {
    if haystack.len() < needle.len() {
        return false;
    }
    let mut i = 0usize;
    while i < needle.len() {
        if haystack[i] != needle[i] {
            return false;
        }
        i += 1;
    }
    true
}

fn http_response(
    bs: *mut EfiBootServices,
    http: *mut EfiHttpProtocol,
    body: *mut u8,
    body_len: &mut usize,
    http_status: &mut u32,
) -> EfiStatus {
    let mut event = null_mut();
    let status = unsafe {
        ((*bs).create_event)(
            EVT_NOTIFY_SIGNAL,
            TPL_CALLBACK,
            Some(noop_event),
            null_mut(),
            &mut event,
        )
    };
    if status.is_error() {
        return status;
    }

    let mut response_data = EfiHttpResponseData { status_code: 0 };
    let mut message = EfiHttpMessage {
        data: EfiHttpMessageData {
            response: &mut response_data,
        },
        header_count: 0,
        headers: null_mut(),
        body_length: *body_len,
        body: body as *mut c_void,
    };
    let mut token = EfiHttpToken {
        event,
        status: EFI_NOT_READY,
        message: &mut message,
    };

    let mut status = unsafe { ((*http).response)(http, &mut token) };
    if !status.is_error() {
        status = poll_http(http, &token);
    }
    *body_len = message.body_length;
    *http_status = response_data.status_code;
    if !message.headers.is_null() {
        unsafe {
            ((*bs).free_pool)(message.headers as *mut c_void);
        }
    }
    unsafe {
        ((*bs).close_event)(event);
    }
    status
}

fn find_key<'a>(json: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let mut p = 0;
    while p < json.len() {
        if json[p] != b'"' {
            p += 1;
            continue;
        }
        let mut i = 0;
        while i < key.len() && p + 1 + i < json.len() && json[p + 1 + i] == key[i] {
            i += 1;
        }
        if i == key.len() && p + 1 + i < json.len() && json[p + 1 + i] == b'"' {
            let mut q = p + 2 + key.len();
            while q < json.len() && matches!(json[q], b' ' | b'\r' | b'\n' | b'\t') {
                q += 1;
            }
            if q < json.len() && json[q] == b':' {
                return Some(&json[q + 1..]);
            }
        }
        p += 1;
    }
    None
}

fn json_string(json: &[u8], key: &[u8], out: &mut [u8]) -> Result<usize, ()> {
    let mut p = find_key(json, key).ok_or(())?;
    while !p.is_empty() && matches!(p[0], b' ' | b'\r' | b'\n' | b'\t') {
        p = &p[1..];
    }
    if p.is_empty() || p[0] != b'"' {
        return Err(());
    }
    p = &p[1..];
    let mut n = 0;
    while !p.is_empty() && p[0] != b'"' {
        if p[0] == b'\\' || n + 1 >= out.len() {
            return Err(());
        }
        out[n] = p[0];
        n += 1;
        p = &p[1..];
    }
    if p.is_empty() || p[0] != b'"' {
        return Err(());
    }
    if n < out.len() {
        out[n] = 0;
    }
    Ok(n)
}

fn parse_u64(mut s: &[u8]) -> Result<u64, ()> {
    let mut value = 0u64;
    let mut radix = 10u64;
    let mut saw = false;
    if s.len() >= 2 && s[0] == b'0' && (s[1] == b'x' || s[1] == b'X') {
        radix = 16;
        s = &s[2..];
    }
    let mut i = 0;
    while i < s.len() {
        if s[i] == b'_' {
            i += 1;
            continue;
        }
        let digit = match s[i] {
            b'0'..=b'9' => s[i] - b'0',
            b'a'..=b'f' => s[i] - b'a' + 10,
            b'A'..=b'F' => s[i] - b'A' + 10,
            _ => break,
        };
        if digit as u64 >= radix {
            return Err(());
        }
        value = value * radix + digit as u64;
        saw = true;
        i += 1;
    }
    if saw {
        Ok(value)
    } else {
        Err(())
    }
}

fn json_u64(json: &[u8], key: &[u8]) -> Result<u64, ()> {
    let mut p = find_key(json, key).ok_or(())?;
    while !p.is_empty() && matches!(p[0], b' ' | b'\r' | b'\n' | b'\t') {
        p = &p[1..];
    }
    parse_u64(p)
}

fn json_addr_string(json: &[u8], key: &[u8]) -> Result<u64, ()> {
    let mut buf = [0u8; 64];
    let len = json_string(json, key, &mut buf)?;
    parse_u64(&buf[..len])
}

fn parse_manifest(json: &[u8]) -> Result<Manifest, ()> {
    let mut manifest = Manifest {
        kernel_url: [0; 1024],
        kernel_url_len: 0,
        kernel_size: 0,
        kernel_load_addr: 0,
        entry_point: 0,
        arch: [0; 32],
        arch_len: 0,
    };
    manifest.kernel_url_len = json_string(json, b"kernel_url", &mut manifest.kernel_url)?;
    manifest.kernel_size = json_u64(json, b"kernel_size")?;
    manifest.kernel_load_addr = json_addr_string(json, b"kernel_load_addr")?;
    manifest.entry_point = json_addr_string(json, b"entry_point")?;
    manifest.arch_len = json_string(json, b"arch", &mut manifest.arch)?;
    Ok(manifest)
}

fn page_count(size: u64) -> usize {
    ((size as usize) + EFI_PAGE_SIZE - 1) / EFI_PAGE_SIZE
}

fn download_kernel(
    bs: *mut EfiBootServices,
    http: *mut EfiHttpProtocol,
    manifest: &Manifest,
) -> EfiStatus {
    if manifest.kernel_size == 0 || manifest.kernel_size > MAX_KERNEL_SIZE {
        return EFI_UNSUPPORTED;
    }
    if (manifest.kernel_load_addr as usize % EFI_PAGE_SIZE) != 0 {
        return EFI_UNSUPPORTED;
    }

    let pages = page_count(manifest.kernel_size);
    let mut target = manifest.kernel_load_addr;
    let status = unsafe {
        ((*bs).allocate_pages)(EFI_ALLOCATE_ADDRESS, EFI_LOADER_DATA, pages, &mut target)
    };
    write_status("kernel_allocate_pages_status: ", status);
    write_ascii("kernel_target_addr: 0x");
    write_hex64(target);
    write_ascii("\r\n");
    if status.is_error() || target != manifest.kernel_load_addr {
        return status;
    }

    let status = http_request(
        bs,
        http,
        &manifest.kernel_url[..manifest.kernel_url_len],
        "kernel",
    );
    write_status("kernel_request_completion: ", status);
    if status.is_error() {
        unsafe {
            ((*bs).free_pages)(target, pages);
        }
        return status;
    }

    let mut downloaded = 0u64;
    let mut checksum = 0u32;
    while downloaded < manifest.kernel_size {
        let remaining = (manifest.kernel_size - downloaded) as usize;
        let mut body_len = if remaining < KERNEL_CHUNK {
            remaining
        } else {
            KERNEL_CHUNK
        };
        let mut http_status = 0u32;
        let dst = (manifest.kernel_load_addr + downloaded) as *mut u8;
        let status = http_response(bs, http, dst, &mut body_len, &mut http_status);
        if status.is_error() || http_status != HTTP_STATUS_200_OK || body_len == 0 {
            write_status("kernel_response_completion: ", status);
            write_ascii("kernel_response_status_enum: ");
            write_dec(http_status as u64);
            write_ascii("\r\n");
            unsafe {
                ((*bs).free_pages)(target, pages);
            }
            return if status.is_error() {
                status
            } else {
                EFI_DEVICE_ERROR
            };
        }
        let slice = unsafe { core::slice::from_raw_parts(dst, body_len) };
        let mut i = 0;
        while i < slice.len() {
            checksum = checksum.wrapping_add(slice[i] as u32);
            i += 1;
        }
        downloaded += body_len as u64;
    }

    write_ascii("kernel_downloaded_size: ");
    write_dec(downloaded);
    write_ascii("\r\n");
    write_ascii("kernel_expected_size: ");
    write_dec(manifest.kernel_size);
    write_ascii("\r\n");
    write_ascii("kernel_checksum32: 0x");
    write_hex64(checksum as u64);
    write_ascii("\r\n");
    EFI_SUCCESS
}

fn print_memory_map(bs: *mut EfiBootServices, map_key_out: &mut usize) -> EfiStatus {
    let mut map_size = MEMORY_MAP_MAX;
    let mut descriptor_size = 0usize;
    let mut descriptor_version = 0u32;
    let memory_map = core::ptr::addr_of_mut!(MEMORY_MAP) as *mut EfiMemoryDescriptor;
    let status = unsafe {
        ((*bs).get_memory_map)(
            &mut map_size,
            memory_map,
            map_key_out,
            &mut descriptor_size,
            &mut descriptor_version,
        )
    };
    write_status("memory_map_status: ", status);
    write_ascii("memory_map_size: ");
    write_dec(map_size as u64);
    write_ascii("\r\n");
    write_ascii("memory_map_key: ");
    write_dec(*map_key_out as u64);
    write_ascii("\r\n");
    write_ascii("memory_map_descriptor_size: ");
    write_dec(descriptor_size as u64);
    write_ascii("\r\n");
    status
}

fn call_kernel(entry_point: u64) -> ! {
    let entry: extern "C" fn() = unsafe { core::mem::transmute(entry_point as usize) };
    entry();
    loop {
        core::hint::spin_loop();
    }
}

fn write_prefixed_status(label: &str, suffix: &str, status: EfiStatus) {
    write_ascii(label);
    write_ascii(suffix);
    write_ascii(": 0x");
    write_hex64(status.0);
    write_ascii("\r\n");
}

fn try_manifest_with_fresh_http_child(
    image: EfiHandle,
    bs: *mut EfiBootServices,
    binding: *mut EfiServiceBindingProtocol,
    url: &[u8],
    label: &str,
    host: Option<&[u8]>,
) -> EfiStatus {
    write_ascii(label);
    write_ascii("_url: ");
    write_bytes(url);
    write_ascii("\r\n");

    let mut child = null_mut();
    let status = unsafe { ((*binding).create_child)(binding, &mut child) };
    write_prefixed_status(label, "_http_create_child_status", status);
    if status.is_error() || child.is_null() {
        return if status.is_error() {
            status
        } else {
            EFI_UNSUPPORTED
        };
    }

    let http = match open_protocol::<EfiHttpProtocol>(bs, child, &EFI_HTTP_PROTOCOL_GUID) {
        Ok(http) => {
            write_prefixed_status(label, "_http_child_protocol_status", EFI_SUCCESS);
            http
        }
        Err(status) => {
            write_prefixed_status(label, "_http_child_protocol_status", status);
            unsafe {
                ((*binding).destroy_child)(binding, child);
            }
            return status;
        }
    };

    let mut tls_label = [0u8; 96];
    let suffix = b"_http_child_tls_config";
    let mut tls_label_len = 0usize;
    while tls_label_len < label.as_bytes().len() && tls_label_len < tls_label.len() {
        tls_label[tls_label_len] = label.as_bytes()[tls_label_len];
        tls_label_len += 1;
    }
    let mut suffix_pos = 0usize;
    while suffix_pos < suffix.len() && tls_label_len < tls_label.len() {
        tls_label[tls_label_len] = suffix[suffix_pos];
        tls_label_len += 1;
        suffix_pos += 1;
    }
    let tls_label_str = unsafe { core::str::from_utf8_unchecked(&tls_label[..tls_label_len]) };
    configure_tls_ca_on_handle(bs, child, tls_label_str);

    print_http_mode_data(http, "http_pre_configure");
    let status = configure_http(http);
    write_prefixed_status(label, "_http_configure_status", status);
    if status.is_error() {
        unsafe {
            ((*binding).destroy_child)(binding, child);
        }
        return status;
    }
    print_http_mode_data(http, "http_post_configure");
    warm_up_http(bs, http);

    let mut manifest_body = [0u8; MANIFEST_MAX + 1];
    let mut body_len = MANIFEST_MAX;
    let mut http_status = 0u32;
    let status = http_get_once_with_host(
        bs,
        http,
        url,
        label,
        host,
        manifest_body.as_mut_ptr(),
        &mut body_len,
        &mut http_status,
    );
    write_status("manifest_response_completion: ", status);
    write_ascii("manifest_response_status_enum: ");
    write_dec(http_status as u64);
    write_ascii("\r\n");
    write_ascii("manifest_response_body_length: ");
    write_dec(body_len as u64);
    write_ascii("\r\n");
    if status.is_error() || http_status != HTTP_STATUS_200_OK || body_len >= manifest_body.len() {
        unsafe {
            ((*binding).destroy_child)(binding, child);
        }
        return if status.is_error() {
            status
        } else {
            EFI_DEVICE_ERROR
        };
    }
    manifest_body[body_len] = 0;

    let manifest = match parse_manifest(&manifest_body[..body_len]) {
        Ok(manifest) => manifest,
        Err(_) => {
            write_ascii("manifest_parse_failed\r\n");
            unsafe {
                ((*binding).destroy_child)(binding, child);
            }
            return EFI_DEVICE_ERROR;
        }
    };

    write_ascii("manifest_arch: ");
    write_bytes(&manifest.arch[..manifest.arch_len]);
    write_ascii("\r\n");
    write_ascii("manifest_kernel_url: ");
    write_bytes(&manifest.kernel_url[..manifest.kernel_url_len]);
    write_ascii("\r\n");
    write_ascii("manifest_kernel_size: ");
    write_dec(manifest.kernel_size);
    write_ascii("\r\n");
    write_ascii("manifest_kernel_load_addr: 0x");
    write_hex64(manifest.kernel_load_addr);
    write_ascii("\r\n");
    write_ascii("manifest_entry_point: 0x");
    write_hex64(manifest.entry_point);
    write_ascii("\r\n");

    let status = download_kernel(bs, http, &manifest);
    write_status("kernel_download_status: ", status);
    if status.is_error() {
        unsafe {
            ((*binding).destroy_child)(binding, child);
        }
        return status;
    }

    let mut map_key = 0usize;
    let status = print_memory_map(bs, &mut map_key);
    write_ascii("boot_jump_enabled: ");
    write_ascii(if OSTOOL_ENABLE_BOOT_JUMP {
        "yes\r\n"
    } else {
        "no\r\n"
    });
    if !OSTOOL_ENABLE_BOOT_JUMP || status.is_error() {
        write_ascii("jump_skipped: boot jump disabled\r\n");
        unsafe {
            ((*binding).destroy_child)(binding, child);
        }
        return EFI_SUCCESS;
    }

    let status = unsafe { ((*bs).exit_boot_services)(image, map_key) };
    if !status.is_error() {
        call_kernel(manifest.entry_point);
    }
    write_status("exit_boot_services_status: ", status);
    write_ascii("jump_failed\r\n");
    unsafe {
        ((*binding).destroy_child)(binding, child);
    }
    EFI_SUCCESS
}

fn try_http_service_handle(
    image: EfiHandle,
    bs: *mut EfiBootServices,
    service_handle: EfiHandle,
    index: usize,
) -> EfiStatus {
    write_ascii("http_service_binding_try_index: ");
    write_dec(index as u64);
    write_ascii("\r\n");

    let binding = match open_protocol::<EfiServiceBindingProtocol>(
        bs,
        service_handle,
        &EFI_HTTP_SERVICE_BINDING_PROTOCOL_GUID,
    ) {
        Ok(binding) => {
            write_status("http_service_binding_open_status: ", EFI_SUCCESS);
            binding
        }
        Err(status) => {
            write_status("http_service_binding_open_status: ", status);
            return status;
        }
    };

    let status = try_manifest_with_fresh_http_child(
        image,
        bs,
        binding,
        OSTOOL_MANIFEST_URL.as_bytes(),
        "manifest",
        None,
    );
    if !status.is_error() {
        return status;
    }

    let host_3443_status = try_manifest_with_fresh_http_child(
        image,
        bs,
        binding,
        OSTOOL_MANIFEST_URL.as_bytes(),
        "manifest_host_3443",
        Some(b"10.3.10.229:3443"),
    );
    if !host_3443_status.is_error() {
        return host_3443_status;
    }

    let mut url443 = [0u8; URL16_MAX];
    match make_https_443_url(OSTOOL_MANIFEST_URL.as_bytes(), &mut url443) {
        Ok(url443_len) => {
            let host_443_status = try_manifest_with_fresh_http_child(
                image,
                bs,
                binding,
                &url443[..url443_len],
                "manifest_host_443",
                Some(b"10.3.10.229"),
            );
            if !host_443_status.is_error() {
                return host_443_status;
            }
        }
        Err(make_status) => {
            write_status("manifest_443_url_status: ", make_status);
        }
    }

    status
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.efi_main")]
extern "C" fn efi_main(image: EfiHandle, system_table: *mut EfiSystemTable) -> EfiStatus {
    unsafe {
        CONSOLE = if system_table.is_null() {
            null_mut()
        } else {
            (*system_table).con_out
        };
    }
    write_ascii("ostool LoongArch64 UEFI loader\r\n");
    write_ascii("manifest_url: ");
    write_ascii(OSTOOL_MANIFEST_URL);
    write_ascii("\r\n");

    let bs = unsafe {
        if system_table.is_null() {
            return EFI_SUCCESS;
        }
        (*system_table).boot_services
    };
    if bs.is_null() {
        return EFI_SUCCESS;
    }

    print_protocol_handle_count(
        bs,
        "tls_service_binding",
        &EFI_TLS_SERVICE_BINDING_PROTOCOL_GUID,
    );
    print_protocol_handle_count(
        bs,
        "tcp4_service_binding",
        &EFI_TCP4_SERVICE_BINDING_PROTOCOL_GUID,
    );
    configure_tls_ca(bs);

    let mut service_count = 0usize;
    let mut service_handles = null_mut();
    let status = locate_protocol_handles(
        bs,
        &EFI_HTTP_SERVICE_BINDING_PROTOCOL_GUID,
        &mut service_count,
        &mut service_handles,
    );
    write_status("http_service_binding_status: ", status);
    write_ascii("http_service_binding_handle_count: ");
    write_dec(service_count as u64);
    write_ascii("\r\n");
    if status.is_error() || service_count == 0 || service_handles.is_null() {
        return EFI_SUCCESS;
    }

    let mut i = 0usize;
    while i < service_count {
        let handle = unsafe { *service_handles.add(i) };
        let status = try_http_service_handle(image, bs, handle, i);
        write_status("http_service_binding_try_status: ", status);
        if !status.is_error() {
            break;
        }
        i += 1;
    }
    unsafe {
        ((*bs).free_pool)(service_handles as *mut c_void);
    }
    EFI_SUCCESS
}

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
