#include <stddef.h>
#include <stdint.h>

#ifndef OSTOOL_MANIFEST_URL
#define OSTOOL_MANIFEST_URL "http://10.3.10.229:2999/boot/boards/loongchip-httpboot-smoke/current/manifest.json"
#endif

#ifndef OSTOOL_ENABLE_BOOT_JUMP
#define OSTOOL_ENABLE_BOOT_JUMP 0
#endif

typedef uint64_t efi_physical_address_t;
typedef uint64_t efi_virtual_address_t;
typedef void *efi_handle_t;
typedef void *efi_event_t;
typedef uint64_t efi_status_t;
typedef uint64_t efi_tpl_t;
typedef uint64_t efi_uintn_t;
typedef uint32_t efi_memory_type_t;
typedef uint32_t efi_allocate_type_t;
typedef uint32_t efi_locate_search_type_t;
typedef uint16_t efi_char16_t;

#define EFI_SUCCESS 0
#define EFI_ERROR_BIT (1ULL << 63)
#define EFI_LOAD_ERROR (EFI_ERROR_BIT | 1)
#define EFI_UNSUPPORTED (EFI_ERROR_BIT | 3)
#define EFI_BUFFER_TOO_SMALL (EFI_ERROR_BIT | 5)
#define EFI_NOT_READY (EFI_ERROR_BIT | 6)
#define EFI_DEVICE_ERROR (EFI_ERROR_BIT | 7)
#define EFI_NOT_FOUND (EFI_ERROR_BIT | 14)
#define EFI_ACCESS_DENIED (EFI_ERROR_BIT | 15)

#define EFI_ALLOCATE_ADDRESS 0
#define EFI_LOADER_DATA 2
#define EFI_LOCATE_BY_PROTOCOL 2
#define EVT_NOTIFY_SIGNAL 0x00000200U
#define TPL_CALLBACK 8
#define EFI_PAGE_SIZE 4096

#define HTTP_VERSION_11 1
#define HTTP_METHOD_GET 0
#define HTTP_STATUS_200_OK 3

#define MANIFEST_MAX 4096
#define URL16_MAX 1024
#define KERNEL_CHUNK 16384
#define MAX_KERNEL_SIZE (256U * 1024U * 1024U)
#define MEMORY_MAP_MAX 65536
#define HTTP_POLL_LIMIT 1000000U

typedef struct {
    uint32_t data1;
    uint16_t data2;
    uint16_t data3;
    uint8_t data4[8];
} efi_guid_t;

typedef struct {
    uint64_t signature;
    uint32_t revision;
    uint32_t header_size;
    uint32_t crc32;
    uint32_t reserved;
} efi_table_header_t;

typedef struct efi_simple_text_output_protocol efi_simple_text_output_protocol_t;
struct efi_simple_text_output_protocol {
    void *reset;
    efi_status_t (*output_string)(efi_simple_text_output_protocol_t *this,
                                  const efi_char16_t *string);
};

typedef struct {
    uint32_t type;
    efi_physical_address_t physical_start;
    efi_virtual_address_t virtual_start;
    uint64_t number_of_pages;
    uint64_t attribute;
} efi_memory_descriptor_t;

typedef struct efi_boot_services efi_boot_services_t;
struct efi_boot_services {
    efi_table_header_t hdr;
    void *raise_tpl;
    void *restore_tpl;
    efi_status_t (*allocate_pages)(efi_allocate_type_t type,
                                   efi_memory_type_t memory_type,
                                   efi_uintn_t pages,
                                   efi_physical_address_t *memory);
    efi_status_t (*free_pages)(efi_physical_address_t memory, efi_uintn_t pages);
    efi_status_t (*get_memory_map)(efi_uintn_t *memory_map_size,
                                   efi_memory_descriptor_t *memory_map,
                                   efi_uintn_t *map_key,
                                   efi_uintn_t *descriptor_size,
                                   uint32_t *descriptor_version);
    efi_status_t (*allocate_pool)(efi_memory_type_t pool_type,
                                  efi_uintn_t size,
                                  void **buffer);
    efi_status_t (*free_pool)(void *buffer);
    efi_status_t (*create_event)(uint32_t type,
                                 efi_tpl_t notify_tpl,
                                 void (*notify_function)(efi_event_t event, void *context),
                                 void *notify_context,
                                 efi_event_t *event);
    void *set_timer;
    void *wait_for_event;
    void *signal_event;
    efi_status_t (*close_event)(efi_event_t event);
    void *check_event;
    void *install_protocol_interface;
    void *reinstall_protocol_interface;
    void *uninstall_protocol_interface;
    efi_status_t (*handle_protocol)(efi_handle_t handle,
                                    const efi_guid_t *protocol,
                                    void **interface);
    void *reserved;
    void *register_protocol_notify;
    void *locate_handle;
    void *locate_device_path;
    void *install_configuration_table;
    void *load_image;
    void *start_image;
    void *exit;
    void *unload_image;
    efi_status_t (*exit_boot_services)(efi_handle_t image_handle,
                                       efi_uintn_t map_key);
    void *get_next_monotonic_count;
    efi_status_t (*stall)(efi_uintn_t microseconds);
    void *set_watchdog_timer;
    void *connect_controller;
    void *disconnect_controller;
    void *open_protocol;
    void *close_protocol;
    void *open_protocol_information;
    void *protocols_per_handle;
    efi_status_t (*locate_handle_buffer)(efi_locate_search_type_t search_type,
                                         const efi_guid_t *protocol,
                                         void *search_key,
                                         efi_uintn_t *no_handles,
                                         efi_handle_t **buffer);
};

typedef struct {
    efi_table_header_t hdr;
    efi_char16_t *firmware_vendor;
    uint32_t firmware_revision;
    efi_handle_t console_in_handle;
    void *con_in;
    efi_handle_t console_out_handle;
    efi_simple_text_output_protocol_t *con_out;
    efi_handle_t standard_error_handle;
    efi_simple_text_output_protocol_t *std_err;
    void *runtime_services;
    efi_boot_services_t *boot_services;
} efi_system_table_t;

typedef struct efi_service_binding_protocol efi_service_binding_protocol_t;
struct efi_service_binding_protocol {
    efi_status_t (*create_child)(efi_service_binding_protocol_t *this,
                                 efi_handle_t *child_handle);
    efi_status_t (*destroy_child)(efi_service_binding_protocol_t *this,
                                  efi_handle_t child_handle);
};

typedef struct efi_http_protocol efi_http_protocol_t;
typedef struct efi_tls_configuration_protocol efi_tls_configuration_protocol_t;
typedef union {
    void *ipv4_node;
    void *ipv6_node;
} efi_http_config_access_point_t;

typedef struct {
    uint8_t use_default_address;
    uint8_t local_address[4];
    uint8_t local_subnet[4];
    uint16_t local_port;
} efi_httpv4_access_point_t;

typedef struct {
    uint32_t http_version;
    uint32_t timeout_millisec;
    uint8_t local_address_is_ipv6;
    uint8_t padding[7];
    efi_http_config_access_point_t access_point;
} efi_http_config_data_t;

typedef struct {
    uint32_t method;
    efi_char16_t *url;
} efi_http_request_data_t;

typedef struct {
    uint32_t status_code;
} efi_http_response_data_t;

typedef union {
    efi_http_request_data_t *request;
    efi_http_response_data_t *response;
} efi_http_message_data_t;

typedef struct {
    char *field_name;
    char *field_value;
} efi_http_header_t;

typedef struct {
    efi_http_message_data_t data;
    efi_uintn_t header_count;
    efi_http_header_t *headers;
    efi_uintn_t body_length;
    void *body;
} efi_http_message_t;

typedef struct {
    efi_event_t event;
    efi_status_t status;
    efi_http_message_t *message;
} efi_http_token_t;

struct efi_http_protocol {
    void *get_mode_data;
    efi_status_t (*configure)(efi_http_protocol_t *this,
                              efi_http_config_data_t *http_config_data);
    efi_status_t (*request)(efi_http_protocol_t *this, efi_http_token_t *token);
    void *cancel;
    efi_status_t (*response)(efi_http_protocol_t *this, efi_http_token_t *token);
    efi_status_t (*poll)(efi_http_protocol_t *this);
};

enum {
    EFI_TLS_CONFIG_DATA_TYPE_HOST_PUBLIC_CERT = 0,
    EFI_TLS_CONFIG_DATA_TYPE_HOST_PRIVATE_KEY = 1,
    EFI_TLS_CONFIG_DATA_TYPE_CA_CERTIFICATE = 2,
    EFI_TLS_CONFIG_DATA_TYPE_CERT_REVOCATION_LIST = 3,
};

struct efi_tls_configuration_protocol {
    efi_status_t (*set_data)(efi_tls_configuration_protocol_t *this,
                             uint32_t data_type,
                             void *data,
                             efi_uintn_t data_size);
    efi_status_t (*get_data)(efi_tls_configuration_protocol_t *this,
                             uint32_t data_type,
                             void *data,
                             efi_uintn_t *data_size);
};

typedef struct {
    char kernel_url[1024];
    uint64_t kernel_size;
    uint64_t kernel_load_addr;
    uint64_t entry_point;
    char arch[32];
} manifest_t;

static const efi_guid_t efi_http_service_binding_protocol_guid = {
    0xbdc8e6af, 0xd9bc, 0x4379, {0xa7, 0x2a, 0xe0, 0xc4, 0xe7, 0x5d, 0xae, 0x1c}
};

static const efi_guid_t efi_http_protocol_guid = {
    0x7a59b29b, 0x910b, 0x4171, {0x82, 0x42, 0xa8, 0x5a, 0x0d, 0xf2, 0x5b, 0x5b}
};

static const efi_guid_t efi_tls_service_binding_protocol_guid = {
    0x952cb795, 0xff36, 0x48cf, {0xa2, 0x49, 0x4d, 0xf4, 0x86, 0xd6, 0xab, 0x8d}
};

static const efi_guid_t efi_tls_configuration_protocol_guid = {
    0x1682fe44, 0xbd7a, 0x4407, {0xb7, 0xc7, 0xdc, 0xa3, 0x7c, 0xa3, 0x92, 0x2d}
};

static const efi_guid_t efi_tcp4_service_binding_protocol_guid = {
    0x00720665, 0x67eb, 0x4a99, {0xba, 0xf7, 0xd3, 0xc3, 0x3a, 0x1c, 0x7c, 0xc9}
};

static efi_simple_text_output_protocol_t *g_console;

static char g_https_ca_pem[] =
    "-----BEGIN CERTIFICATE-----\n"
    "MIIDHjCCAgagAwIBAgIUBJubHQIousJm3ZT9sNPYq0u1AhwwDQYJKoZIhvcNAQEL\n"
    "BQAwFjEUMBIGA1UEAwwLMTAuMy4xMC4yMjkwHhcNMjYwNTEzMDkxMTIwWhcNMjYw\n"
    "NTIwMDkxMTIwWjAWMRQwEgYDVQQDDAsxMC4zLjEwLjIyOTCCASIwDQYJKoZIhvcN\n"
    "AQEBBQADggEPADCCAQoCggEBALb3klUcff8fXYIcsgeQr1gs2rnwbOl/4Unwtulx\n"
    "wG1K8joXYwWT4NP4XSOJy8aVuLk0FSd8VB29l6gjduSYzdC1CE9i3bzJnu4E96X/\n"
    "EqRWP6QPkQJUizpH3qwxK1sDNJTmoAdq48v3cLgyyDdxzU/iVlWM51izk4njFMzZ\n"
    "4PCLFfznANnj8o5diDCQ96uyKxmaXArIeDAAwcTSlJYc7QHWg6WEg+FQcn3TaMKJ\n"
    "8rNELrKMrygc71ZdF9r6anud4YMouse6wJmEzGEVSCQ/y3dxd8gr1Ixq3DV9Yj9J\n"
    "fZ57GWjWLTi1CWyMAAod8b6xr+o2yHRS2mGIfTEskTHTc+8CAwEAAaNkMGIwHQYD\n"
    "VR0OBBYEFIimL7eY9PEocEUee1gz/YxTalmbMB8GA1UdIwQYMBaAFIimL7eY9PEo\n"
    "cEUee1gz/YxTalmbMA8GA1UdEwEB/wQFMAMBAf8wDwYDVR0RBAgwBocECgMK5TAN\n"
    "BgkqhkiG9w0BAQsFAAOCAQEAf1TkdDogQAYDgSUdraZ6WtOrD7MrLH69DZIcMrVf\n"
    "GymOgar70uD9s1MEAwAsCgfqN8+kRcR/viWY8e86AzYVralqiLVs9tpR+vrnFejd\n"
    "f9KLftc3owFwmiMLR5szwZMENOz2F+TJ8fNZBTXaJuITxrcIwuBym0FqL1pkN4hL\n"
    "ikfU5paqfDst5LA/Wu/56XPtP8tFGh498jNsKlAumlQgaX0w+xxGiaGf1WkvTOP8\n"
    "bVwyUYVeTIG2utpOKra0gkg42qcPdvRzZsT9REzlp2cxyBx5fkSmS0kXtqg4fT69\n"
    "9/dZPxTWismXdZ4HN74kKak5tAB9CwKXvwaLRqOmXEg4hw==\n"
    "-----END CERTIFICATE-----\n";

static size_t strlen8(const char *s) {
    size_t len = 0;
    while (s[len] != '\0') {
        len++;
    }
    return len;
}

static void memset8(void *dst, uint8_t value, size_t len) {
    uint8_t *d = (uint8_t *)dst;
    for (size_t i = 0; i < len; i++) {
        d[i] = value;
    }
}

static int is_error(efi_status_t status) {
    return (status & EFI_ERROR_BIT) != 0;
}

static void write_ascii(const char *s) {
    static efi_char16_t buf[256];
    if (!g_console) {
        return;
    }
    while (*s) {
        size_t n = 0;
        while (s[n] && n + 1 < (sizeof(buf) / sizeof(buf[0]))) {
            buf[n] = (uint8_t)s[n];
            n++;
        }
        buf[n] = 0;
        g_console->output_string(g_console, buf);
        s += n;
    }
}

static void write_hex64(uint64_t value) {
    char out[17];
    for (int i = 0; i < 16; i++) {
        uint8_t digit = (value >> ((15 - i) * 4)) & 0xf;
        out[i] = (digit < 10) ? ('0' + digit) : ('a' + digit - 10);
    }
    out[16] = '\0';
    write_ascii(out);
}

static void write_dec(uint64_t value) {
    char out[32];
    size_t pos = sizeof(out);
    out[--pos] = '\0';
    if (value == 0) {
        out[--pos] = '0';
    } else {
        while (value > 0 && pos > 0) {
            out[--pos] = (char)('0' + (value % 10));
            value /= 10;
        }
    }
    write_ascii(&out[pos]);
}

static void write_status(const char *label, efi_status_t status) {
    write_ascii(label);
    write_ascii("0x");
    write_hex64(status);
    write_ascii("\r\n");
}

static int write_utf16_url(const char *url, efi_char16_t *out, size_t cap) {
    size_t len = strlen8(url);
    if (len + 1 > cap) {
        return -1;
    }
    for (size_t i = 0; i < len; i++) {
        out[i] = (uint8_t)url[i];
    }
    out[len] = 0;
    return 0;
}

static int write_https_probe_url(const char *url, efi_char16_t *out, size_t cap) {
    const char http_prefix[] = "http://";
    const char https_prefix[] = "https://";
    for (size_t i = 0; i < sizeof(http_prefix) - 1; i++) {
        if (url[i] != http_prefix[i]) {
            return -1;
        }
    }

    size_t suffix_len = strlen8(url + sizeof(http_prefix) - 1);
    if ((sizeof(https_prefix) - 1) + suffix_len + 1 > cap) {
        return -1;
    }
    size_t pos = 0;
    for (size_t i = 0; i < sizeof(https_prefix) - 1; i++) {
        out[pos++] = (uint8_t)https_prefix[i];
    }
    for (size_t i = 0; i < suffix_len; i++) {
        out[pos++] = (uint8_t)url[(sizeof(http_prefix) - 1) + i];
    }
    out[pos] = 0;
    return 0;
}

static void noop_event(efi_event_t event, void *context) {
    (void)event;
    (void)context;
}

static efi_status_t poll_http(efi_http_protocol_t *http, efi_http_token_t *token) {
    volatile efi_status_t *token_status = &token->status;
    for (uint32_t i = 0; i < HTTP_POLL_LIMIT; i++) {
        if (*token_status != EFI_NOT_READY) {
            return *token_status;
        }
        http->poll(http);
    }
    return *token_status;
}

static void warm_up_http(efi_boot_services_t *bs, efi_http_protocol_t *http) {
    efi_status_t last_poll = EFI_NOT_READY;
    for (uint32_t i = 0; i < 20; i++) {
        if (bs->stall) {
            bs->stall(100000);
        }
        last_poll = http->poll(http);
    }
    write_status("http_post_configure_poll_status: ", last_poll);
}

static efi_status_t locate_protocol_handles(efi_boot_services_t *bs,
                                            const efi_guid_t *guid,
                                            efi_uintn_t *count,
                                            efi_handle_t **handles) {
    *count = 0;
    *handles = 0;
    return bs->locate_handle_buffer(EFI_LOCATE_BY_PROTOCOL, guid, 0, count, handles);
}

static void print_protocol_handle_count(efi_boot_services_t *bs,
                                        const char *label,
                                        const efi_guid_t *guid) {
    efi_uintn_t count = 0;
    efi_handle_t *handles = 0;
    efi_status_t status = locate_protocol_handles(bs, guid, &count, &handles);
    write_ascii(label);
    write_ascii("_status: ");
    write_ascii("0x");
    write_hex64(status);
    write_ascii("\r\n");
    write_ascii(label);
    write_ascii("_handle_count: ");
    write_dec(count);
    write_ascii("\r\n");
    if (!is_error(status) && handles) {
        bs->free_pool(handles);
    }
}

static void configure_tls_ca(efi_boot_services_t *bs) {
    efi_uintn_t service_count = 0;
    efi_handle_t *service_handles = 0;
    efi_status_t status = locate_protocol_handles(bs,
                                                  &efi_tls_service_binding_protocol_guid,
                                                  &service_count,
                                                  &service_handles);
    write_status("tls_config_service_status: ", status);
    write_ascii("tls_config_service_handle_count: ");
    write_dec(service_count);
    write_ascii("\r\n");
    if (is_error(status) || service_count == 0 || !service_handles) {
        return;
    }

    efi_service_binding_protocol_t *binding = 0;
    status = bs->handle_protocol(service_handles[0],
                                 &efi_tls_service_binding_protocol_guid,
                                 (void **)&binding);
    write_status("tls_config_binding_open_status: ", status);
    if (is_error(status) || !binding) {
        bs->free_pool(service_handles);
        return;
    }

    efi_handle_t child = 0;
    status = binding->create_child(binding, &child);
    write_status("tls_config_create_child_status: ", status);
    if (is_error(status) || !child) {
        bs->free_pool(service_handles);
        return;
    }

    efi_tls_configuration_protocol_t *tls_config = 0;
    status = bs->handle_protocol(child,
                                 &efi_tls_configuration_protocol_guid,
                                 (void **)&tls_config);
    write_status("tls_config_protocol_status: ", status);
    if (!is_error(status) && tls_config) {
        efi_uintn_t ca_size = 0;
        efi_status_t get_status = tls_config->get_data(tls_config,
                                                       EFI_TLS_CONFIG_DATA_TYPE_CA_CERTIFICATE,
                                                       0,
                                                       &ca_size);
        write_status("tls_config_get_ca_status: ", get_status);
        write_ascii("tls_config_get_ca_size: ");
        write_dec(ca_size);
        write_ascii("\r\n");

        status = tls_config->set_data(tls_config,
                                      EFI_TLS_CONFIG_DATA_TYPE_CA_CERTIFICATE,
                                      g_https_ca_pem,
                                      (efi_uintn_t)strlen8(g_https_ca_pem));
        write_status("tls_config_set_ca_status: ", status);

        ca_size = 0;
        get_status = tls_config->get_data(tls_config,
                                          EFI_TLS_CONFIG_DATA_TYPE_CA_CERTIFICATE,
                                          0,
                                          &ca_size);
        write_status("tls_config_get_ca_after_status: ", get_status);
        write_ascii("tls_config_get_ca_after_size: ");
        write_dec(ca_size);
        write_ascii("\r\n");
    }

    status = binding->destroy_child(binding, child);
    write_status("tls_config_destroy_child_status: ", status);
    bs->free_pool(service_handles);
}

static efi_status_t configure_http(efi_http_protocol_t *http) {
    static efi_httpv4_access_point_t ipv4;
    static efi_http_config_data_t config;
    memset8(&ipv4, 0, sizeof(ipv4));
    memset8(&config, 0, sizeof(config));
    ipv4.use_default_address = 1;
    config.http_version = HTTP_VERSION_11;
    config.timeout_millisec = 0;
    config.local_address_is_ipv6 = 0;
    config.access_point.ipv4_node = &ipv4;
    return http->configure(http, &config);
}

static efi_status_t http_request(efi_boot_services_t *bs,
                                 efi_http_protocol_t *http,
                                 const char *url,
                                 const char *label) {
    static efi_char16_t url16[URL16_MAX];
    if (write_utf16_url(url, url16, URL16_MAX) != 0) {
        return EFI_BUFFER_TOO_SMALL;
    }

    efi_event_t event = 0;
    efi_status_t status = bs->create_event(EVT_NOTIFY_SIGNAL, TPL_CALLBACK, noop_event, 0, &event);
    write_ascii(label);
    write_ascii("_request_event_status: ");
    write_ascii("0x");
    write_hex64(status);
    write_ascii("\r\n");
    if (is_error(status)) {
        return status;
    }

    efi_http_request_data_t request_data;
    request_data.method = HTTP_METHOD_GET;
    request_data.url = url16;
    efi_http_message_t message;
    memset8(&message, 0, sizeof(message));
    message.data.request = &request_data;
    efi_http_token_t token;
    token.event = event;
    token.status = EFI_NOT_READY;
    token.message = &message;

    efi_status_t submit_status = http->request(http, &token);
    write_ascii(label);
    write_ascii("_request_submit_status: ");
    write_ascii("0x");
    write_hex64(submit_status);
    write_ascii("\r\n");
    status = submit_status;
    if (!is_error(submit_status)) {
        status = poll_http(http, &token);
    }
    write_ascii(label);
    write_ascii("_request_token_status: ");
    write_ascii("0x");
    write_hex64(token.status);
    write_ascii("\r\n");
    bs->close_event(event);
    return status;
}

static efi_status_t https_request_probe(efi_boot_services_t *bs,
                                        efi_http_protocol_t *http,
                                        const char *url) {
    static efi_char16_t url16[URL16_MAX];
    if (write_https_probe_url(url, url16, URL16_MAX) != 0) {
        write_ascii("https_probe_url_status: unavailable\r\n");
        return EFI_UNSUPPORTED;
    }

    efi_event_t event = 0;
    efi_status_t status = bs->create_event(EVT_NOTIFY_SIGNAL, TPL_CALLBACK, noop_event, 0, &event);
    write_status("https_probe_event_status: ", status);
    if (is_error(status)) {
        return status;
    }

    efi_http_request_data_t request_data;
    request_data.method = HTTP_METHOD_GET;
    request_data.url = url16;
    efi_http_message_t message;
    memset8(&message, 0, sizeof(message));
    message.data.request = &request_data;
    efi_http_token_t token;
    token.event = event;
    token.status = EFI_NOT_READY;
    token.message = &message;

    efi_status_t submit_status = http->request(http, &token);
    write_status("https_probe_submit_status: ", submit_status);
    write_status("https_probe_token_status: ", token.status);
    bs->close_event(event);
    return submit_status;
}

static efi_status_t http_response(efi_boot_services_t *bs,
                                  efi_http_protocol_t *http,
                                  void *body,
                                  size_t *body_len,
                                  uint32_t *http_status) {
    efi_event_t event = 0;
    efi_status_t status = bs->create_event(EVT_NOTIFY_SIGNAL, TPL_CALLBACK, noop_event, 0, &event);
    if (is_error(status)) {
        return status;
    }

    efi_http_response_data_t response_data;
    response_data.status_code = 0;
    efi_http_message_t message;
    memset8(&message, 0, sizeof(message));
    message.data.response = &response_data;
    message.body_length = *body_len;
    message.body = body;

    efi_http_token_t token;
    token.event = event;
    token.status = EFI_NOT_READY;
    token.message = &message;

    status = http->response(http, &token);
    if (!is_error(status)) {
        status = poll_http(http, &token);
    }
    *body_len = message.body_length;
    *http_status = response_data.status_code;
    if (message.headers) {
        bs->free_pool(message.headers);
    }
    bs->close_event(event);
    return status;
}

static const char *find_key(const char *json, const char *key) {
    size_t key_len = strlen8(key);
    for (const char *p = json; *p; p++) {
        if (*p != '"') {
            continue;
        }
        size_t i = 0;
        while (i < key_len && p[1 + i] == key[i]) {
            i++;
        }
        if (i == key_len && p[1 + i] == '"') {
            const char *q = p + 2 + key_len;
            while (*q == ' ' || *q == '\r' || *q == '\n' || *q == '\t') {
                q++;
            }
            if (*q == ':') {
                return q + 1;
            }
        }
    }
    return 0;
}

static int json_string(const char *json, const char *key, char *out, size_t cap) {
    const char *p = find_key(json, key);
    if (!p) {
        return -1;
    }
    while (*p == ' ' || *p == '\r' || *p == '\n' || *p == '\t') {
        p++;
    }
    if (*p != '"') {
        return -1;
    }
    p++;
    size_t n = 0;
    while (*p && *p != '"') {
        if (*p == '\\' || n + 1 >= cap) {
            return -1;
        }
        out[n++] = *p++;
    }
    if (*p != '"') {
        return -1;
    }
    out[n] = '\0';
    return 0;
}

static int parse_u64(const char *s, uint64_t *out) {
    uint64_t value = 0;
    int radix = 10;
    int saw = 0;
    if (s[0] == '0' && (s[1] == 'x' || s[1] == 'X')) {
        radix = 16;
        s += 2;
    }
    while (*s) {
        if (*s == '_') {
            s++;
            continue;
        }
        uint8_t digit;
        if (*s >= '0' && *s <= '9') {
            digit = (uint8_t)(*s - '0');
        } else if (*s >= 'a' && *s <= 'f') {
            digit = (uint8_t)(*s - 'a' + 10);
        } else if (*s >= 'A' && *s <= 'F') {
            digit = (uint8_t)(*s - 'A' + 10);
        } else {
            break;
        }
        if (digit >= radix) {
            return -1;
        }
        value = value * (uint64_t)radix + digit;
        saw = 1;
        s++;
    }
    if (!saw) {
        return -1;
    }
    *out = value;
    return 0;
}

static int json_u64(const char *json, const char *key, uint64_t *out) {
    const char *p = find_key(json, key);
    if (!p) {
        return -1;
    }
    while (*p == ' ' || *p == '\r' || *p == '\n' || *p == '\t') {
        p++;
    }
    return parse_u64(p, out);
}

static int json_addr_string(const char *json, const char *key, uint64_t *out) {
    char buf[64];
    if (json_string(json, key, buf, sizeof(buf)) != 0) {
        return -1;
    }
    return parse_u64(buf, out);
}

static int parse_manifest(const char *json, manifest_t *manifest) {
    if (json_string(json, "kernel_url", manifest->kernel_url, sizeof(manifest->kernel_url)) != 0) {
        return -1;
    }
    if (json_u64(json, "kernel_size", &manifest->kernel_size) != 0) {
        return -1;
    }
    if (json_addr_string(json, "kernel_load_addr", &manifest->kernel_load_addr) != 0) {
        return -1;
    }
    if (json_addr_string(json, "entry_point", &manifest->entry_point) != 0) {
        return -1;
    }
    if (json_string(json, "arch", manifest->arch, sizeof(manifest->arch)) != 0) {
        return -1;
    }
    return 0;
}

static efi_uintn_t page_count(uint64_t addr, uint64_t size) {
    (void)addr;
    return (efi_uintn_t)((size + EFI_PAGE_SIZE - 1) / EFI_PAGE_SIZE);
}

static efi_status_t download_kernel(efi_boot_services_t *bs,
                                    efi_http_protocol_t *http,
                                    const manifest_t *manifest,
                                    efi_uintn_t *pages_out) {
    if (manifest->kernel_size == 0 || manifest->kernel_size > MAX_KERNEL_SIZE) {
        return EFI_UNSUPPORTED;
    }
    if ((manifest->kernel_load_addr % EFI_PAGE_SIZE) != 0) {
        return EFI_UNSUPPORTED;
    }

    efi_uintn_t pages = page_count(manifest->kernel_load_addr, manifest->kernel_size);
    efi_physical_address_t target = manifest->kernel_load_addr;
    efi_status_t status = bs->allocate_pages(EFI_ALLOCATE_ADDRESS, EFI_LOADER_DATA, pages, &target);
    write_status("kernel_allocate_pages_status: ", status);
    write_ascii("kernel_target_addr: 0x");
    write_hex64(target);
    write_ascii("\r\n");
    if (is_error(status) || target != manifest->kernel_load_addr) {
        return status;
    }

    status = http_request(bs, http, manifest->kernel_url, "kernel");
    write_status("kernel_request_completion: ", status);
    if (is_error(status)) {
        bs->free_pages(target, pages);
        return status;
    }

    uint64_t downloaded = 0;
    uint32_t checksum = 0;
    while (downloaded < manifest->kernel_size) {
        size_t remaining = (size_t)(manifest->kernel_size - downloaded);
        size_t body_len = remaining < KERNEL_CHUNK ? remaining : KERNEL_CHUNK;
        uint32_t http_status = 0;
        uint8_t *dst = (uint8_t *)(uintptr_t)(manifest->kernel_load_addr + downloaded);
        status = http_response(bs, http, dst, &body_len, &http_status);
        if (is_error(status) || http_status != HTTP_STATUS_200_OK || body_len == 0) {
            write_status("kernel_response_completion: ", status);
            write_ascii("kernel_response_status_enum: ");
            write_dec(http_status);
            write_ascii("\r\n");
            bs->free_pages(target, pages);
            return is_error(status) ? status : EFI_DEVICE_ERROR;
        }
        for (size_t i = 0; i < body_len; i++) {
            checksum += dst[i];
        }
        downloaded += body_len;
    }

    write_ascii("kernel_downloaded_size: ");
    write_dec(downloaded);
    write_ascii("\r\n");
    write_ascii("kernel_expected_size: ");
    write_dec(manifest->kernel_size);
    write_ascii("\r\n");
    write_ascii("kernel_checksum32: 0x");
    write_hex64(checksum);
    write_ascii("\r\n");
    *pages_out = pages;
    return EFI_SUCCESS;
}

static efi_status_t print_memory_map(efi_boot_services_t *bs, efi_uintn_t *map_key_out) {
    static uint8_t memory_map[MEMORY_MAP_MAX];
    efi_uintn_t map_size = sizeof(memory_map);
    efi_uintn_t descriptor_size = 0;
    uint32_t descriptor_version = 0;
    efi_status_t status = bs->get_memory_map(&map_size,
                                             (efi_memory_descriptor_t *)memory_map,
                                             map_key_out,
                                             &descriptor_size,
                                             &descriptor_version);
    write_status("memory_map_status: ", status);
    write_ascii("memory_map_size: ");
    write_dec(map_size);
    write_ascii("\r\n");
    write_ascii("memory_map_key: ");
    write_dec(*map_key_out);
    write_ascii("\r\n");
    write_ascii("memory_map_descriptor_size: ");
    write_dec(descriptor_size);
    write_ascii("\r\n");
    return status;
}

static void call_kernel(uint64_t entry_point) {
    void (*entry)(void) = (void (*)(void))(uintptr_t)entry_point;
    entry();
    for (;;) {
    }
}

static efi_status_t try_http_service_handle(efi_handle_t image,
                                            efi_boot_services_t *bs,
                                            efi_handle_t service_handle,
                                            efi_uintn_t index) {
    write_ascii("http_service_binding_try_index: ");
    write_dec(index);
    write_ascii("\r\n");
    efi_service_binding_protocol_t *binding = 0;
    efi_status_t status = bs->handle_protocol(service_handle,
                                              &efi_http_service_binding_protocol_guid,
                                              (void **)&binding);
    write_status("http_service_binding_open_status: ", status);
    if (is_error(status) || !binding) {
        return is_error(status) ? status : EFI_UNSUPPORTED;
    }

    efi_handle_t child = 0;
    status = binding->create_child(binding, &child);
    write_status("http_create_child_status: ", status);
    if (is_error(status) || !child) {
        return is_error(status) ? status : EFI_UNSUPPORTED;
    }

    efi_http_protocol_t *http = 0;
    status = bs->handle_protocol(child, &efi_http_protocol_guid, (void **)&http);
    write_status("http_child_protocol_status: ", status);
    if (is_error(status) || !http) {
        binding->destroy_child(binding, child);
        return is_error(status) ? status : EFI_UNSUPPORTED;
    }

    status = configure_http(http);
    write_status("http_configure_status: ", status);
    if (is_error(status)) {
        binding->destroy_child(binding, child);
        return status;
    }
    warm_up_http(bs, http);

    static char manifest_body[MANIFEST_MAX + 1];
    status = http_request(bs, http, OSTOOL_MANIFEST_URL, "manifest");
    write_status("manifest_request_completion: ", status);
    if (is_error(status)) {
        if (status == EFI_ACCESS_DENIED) {
            https_request_probe(bs, http, OSTOOL_MANIFEST_URL);
        }
        binding->destroy_child(binding, child);
        return status;
    }

    size_t body_len = MANIFEST_MAX;
    uint32_t http_status = 0;
    status = http_response(bs, http, manifest_body, &body_len, &http_status);
    write_status("manifest_response_completion: ", status);
    write_ascii("manifest_response_status_enum: ");
    write_dec(http_status);
    write_ascii("\r\n");
    write_ascii("manifest_response_body_length: ");
    write_dec(body_len);
    write_ascii("\r\n");
    if (is_error(status) || http_status != HTTP_STATUS_200_OK || body_len >= sizeof(manifest_body)) {
        binding->destroy_child(binding, child);
        return is_error(status) ? status : EFI_DEVICE_ERROR;
    }
    manifest_body[body_len] = '\0';

    manifest_t manifest;
    memset8(&manifest, 0, sizeof(manifest));
    if (parse_manifest(manifest_body, &manifest) != 0) {
        write_ascii("manifest_parse_failed\r\n");
        binding->destroy_child(binding, child);
        return EFI_DEVICE_ERROR;
    }

    write_ascii("manifest_arch: ");
    write_ascii(manifest.arch);
    write_ascii("\r\n");
    write_ascii("manifest_kernel_url: ");
    write_ascii(manifest.kernel_url);
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

    efi_uintn_t kernel_pages = 0;
    status = download_kernel(bs, http, &manifest, &kernel_pages);
    write_status("kernel_download_status: ", status);
    if (is_error(status)) {
        binding->destroy_child(binding, child);
        return status;
    }

    efi_uintn_t map_key = 0;
    status = print_memory_map(bs, &map_key);
    write_ascii("boot_jump_enabled: ");
    write_ascii(OSTOOL_ENABLE_BOOT_JUMP ? "yes\r\n" : "no\r\n");
    if (!OSTOOL_ENABLE_BOOT_JUMP || is_error(status)) {
        write_ascii("jump_skipped: boot jump disabled\r\n");
        binding->destroy_child(binding, child);
        return EFI_SUCCESS;
    }

    status = bs->exit_boot_services(image, map_key);
    if (!is_error(status)) {
        call_kernel(manifest.entry_point);
    }
    write_status("exit_boot_services_status: ", status);
    write_ascii("jump_failed\r\n");
    (void)kernel_pages;
    binding->destroy_child(binding, child);
    return EFI_SUCCESS;
}

efi_status_t efi_main(efi_handle_t image, efi_system_table_t *system_table) {
    g_console = system_table ? system_table->con_out : 0;
    write_ascii("ostool LoongArch64 UEFI loader\r\n");
    write_ascii("manifest_url: ");
    write_ascii(OSTOOL_MANIFEST_URL);
    write_ascii("\r\n");

    efi_boot_services_t *bs = system_table->boot_services;
    print_protocol_handle_count(bs, "tls_service_binding", &efi_tls_service_binding_protocol_guid);
    print_protocol_handle_count(bs, "tcp4_service_binding", &efi_tcp4_service_binding_protocol_guid);
    configure_tls_ca(bs);

    efi_uintn_t service_count = 0;
    efi_handle_t *service_handles = 0;
    efi_status_t status = locate_protocol_handles(bs,
                                                  &efi_http_service_binding_protocol_guid,
                                                  &service_count,
                                                  &service_handles);
    write_status("http_service_binding_status: ", status);
    write_ascii("http_service_binding_handle_count: ");
    write_dec(service_count);
    write_ascii("\r\n");
    if (is_error(status) || service_count == 0 || !service_handles) {
        return EFI_SUCCESS;
    }

    for (efi_uintn_t i = 0; i < service_count; i++) {
        status = try_http_service_handle(image, bs, service_handles[i], i);
        write_status("http_service_binding_try_status: ", status);
        if (!is_error(status)) {
            break;
        }
    }
    bs->free_pool(service_handles);
    return EFI_SUCCESS;
}
