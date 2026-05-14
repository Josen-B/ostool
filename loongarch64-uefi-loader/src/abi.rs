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
const TCP4_POLL_LIMIT: usize = 5_000_000;

const EFI_TLS_CONFIG_DATA_TYPE_CA_CERTIFICATE: u32 = 2;
const EFI_TLS_SESSION_DATA_TYPE_VERSION: u32 = 0;
const EFI_TLS_SESSION_DATA_TYPE_CONNECTION_END: u32 = 1;
const EFI_TLS_SESSION_DATA_TYPE_VERIFY_METHOD: u32 = 5;
const EFI_TLS_SESSION_DATA_TYPE_SESSION_STATE: u32 = 7;
const EFI_TLS_CONNECTION_END_CLIENT: u32 = 0;
const EFI_TLS_VERIFY_NONE: u32 = 0;
const EFI_TLS_SESSION_DATA_TRANSFERRING: u32 = 2;
const VERBOSE_SETUP_LOGS: bool = false;

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

#[repr(C)]
#[derive(Clone, Copy)]
struct EfiTlsVersion {
    major: u8,
    minor: u8,
}

#[repr(C)]
struct EfiTlsProtocol {
    set_session_data: extern "C" fn(*mut EfiTlsProtocol, u32, *mut c_void, usize) -> EfiStatus,
    get_session_data: extern "C" fn(*mut EfiTlsProtocol, u32, *mut c_void, *mut usize) -> EfiStatus,
    build_response_packet:
        extern "C" fn(*mut EfiTlsProtocol, *mut u8, usize, *mut u8, *mut usize) -> EfiStatus,
    process_packet: extern "C" fn(
        *mut EfiTlsProtocol,
        *mut *mut EfiTlsFragmentData,
        *mut u32,
        u32,
    ) -> EfiStatus,
}

#[repr(C)]
struct EfiTlsFragmentData {
    fragment_length: u32,
    fragment_buffer: *mut c_void,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct EfiTcp4AccessPoint {
    use_default_address: u8,
    station_address: [u8; 4],
    subnet_mask: [u8; 4],
    station_port: u16,
    remote_address: [u8; 4],
    remote_port: u16,
    active_flag: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct EfiTcp4Option {
    receive_buffer_size: u32,
    send_buffer_size: u32,
    max_syn_back_log: u32,
    connection_timeout: u32,
    data_retries: u32,
    fin_timeout: u32,
    time_wait_timeout: u32,
    keep_alive_probes: u32,
    keep_alive_time: u32,
    keep_alive_interval: u32,
    enable_nagle: u8,
    enable_time_stamp: u8,
    enable_window_scaling: u8,
    enable_selective_ack: u8,
    enable_path_mtu_discovery: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct EfiTcp4ConfigData {
    type_of_service: u8,
    time_to_live: u8,
    access_point: EfiTcp4AccessPoint,
    control_option: EfiTcp4Option,
}

#[repr(C)]
struct EfiTcp4CompletionToken {
    event: EfiEvent,
    status: EfiStatus,
}

#[repr(C)]
struct EfiTcp4ConnectionToken {
    completion_token: EfiTcp4CompletionToken,
}

#[repr(C)]
struct EfiTcp4CloseToken {
    completion_token: EfiTcp4CompletionToken,
    abort_on_close: u8,
}

#[repr(C)]
struct EfiTcp4FragmentData {
    fragment_length: u32,
    fragment_buffer: *mut c_void,
}

#[repr(C)]
struct EfiTcp4TransmitData {
    push: u8,
    urgent: u8,
    data_length: u32,
    fragment_count: u32,
    fragment_table: [EfiTcp4FragmentData; 1],
}

#[repr(C)]
struct EfiTcp4ReceiveData {
    urgent_flag: u8,
    data_length: u32,
    fragment_count: u32,
    fragment_table: [EfiTcp4FragmentData; 1],
}

#[repr(C)]
union EfiTcp4IoPacket {
    rx_data: *mut EfiTcp4ReceiveData,
    tx_data: *mut EfiTcp4TransmitData,
}

#[repr(C)]
struct EfiTcp4IoToken {
    completion_token: EfiTcp4CompletionToken,
    packet: EfiTcp4IoPacket,
}

#[repr(C)]
struct EfiTcp4Protocol {
    get_mode_data: extern "C" fn(
        *mut EfiTcp4Protocol,
        *mut u32,
        *mut EfiTcp4ConfigData,
        *mut c_void,
        *mut c_void,
        *mut c_void,
    ) -> EfiStatus,
    configure: extern "C" fn(*mut EfiTcp4Protocol, *mut EfiTcp4ConfigData) -> EfiStatus,
    routes: usize,
    connect: extern "C" fn(*mut EfiTcp4Protocol, *mut EfiTcp4ConnectionToken) -> EfiStatus,
    accept: usize,
    transmit: extern "C" fn(*mut EfiTcp4Protocol, *mut EfiTcp4IoToken) -> EfiStatus,
    receive: extern "C" fn(*mut EfiTcp4Protocol, *mut EfiTcp4IoToken) -> EfiStatus,
    close: extern "C" fn(*mut EfiTcp4Protocol, *mut EfiTcp4CloseToken) -> EfiStatus,
    cancel: usize,
    poll: extern "C" fn(*mut EfiTcp4Protocol) -> EfiStatus,
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

const EFI_TCP4_PROTOCOL_GUID: EfiGuid = EfiGuid {
    data1: 0x65530bc7,
    data2: 0xa359,
    data3: 0x410f,
    data4: [0xb0, 0x10, 0x5a, 0xad, 0xc7, 0xec, 0x2b, 0x62],
};

const EFI_TLS_PROTOCOL_GUID: EfiGuid = EfiGuid {
    data1: 0x00ca_959f,
    data2: 0x6cfa,
    data3: 0x4db1,
    data4: [0x95, 0xbc, 0xe4, 0x6c, 0x47, 0x51, 0x43, 0x90],
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
