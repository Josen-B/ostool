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
    if VERBOSE_SETUP_LOGS {
        if label == "tls_config" {
            write_status("tls_config_set_ca_status: ", set_pem_status);
        } else {
            write_ascii(label);
            write_ascii("_set_ca_pem_status: 0x");
            write_hex64(set_pem_status.0);
            write_ascii("\r\n");
        }
    }

    let mut der = [0u8; 1536];
    match pem_to_der(HTTPS_CA_PEM, &mut der) {
        Ok(der_len) => {
            let set_der_status = unsafe {
                ((*tls_config).set_data)(
                    tls_config,
                    EFI_TLS_CONFIG_DATA_TYPE_CA_CERTIFICATE,
                    der.as_mut_ptr() as *mut c_void,
                    der_len,
                )
            };
            if VERBOSE_SETUP_LOGS {
                write_ascii(label);
                write_ascii("_ca_der_size: ");
                write_dec(der_len as u64);
                write_ascii("\r\n");
                write_ascii(label);
                write_ascii("_set_ca_der_status: 0x");
                write_hex64(set_der_status.0);
                write_ascii("\r\n");
            }
        }
        Err(status) => {
            if VERBOSE_SETUP_LOGS {
                write_ascii(label);
                write_ascii("_ca_der_decode_status: 0x");
                write_hex64(status.0);
                write_ascii("\r\n");
            }
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
    if VERBOSE_SETUP_LOGS {
        write_status("tls_config_service_status: ", status);
        write_ascii("tls_config_service_handle_count: ");
        write_dec(service_count as u64);
        write_ascii("\r\n");
    }
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
            if VERBOSE_SETUP_LOGS {
                write_status("tls_config_binding_open_status: ", EFI_SUCCESS);
            }
            binding
        }
        Err(status) => {
            if VERBOSE_SETUP_LOGS {
                write_status("tls_config_binding_open_status: ", status);
            }
            unsafe {
                ((*bs).free_pool)(service_handles as *mut c_void);
            }
            return;
        }
    };

    let mut child = null_mut();
    let status = unsafe { ((*binding).create_child)(binding, &mut child) };
    if VERBOSE_SETUP_LOGS {
        write_status("tls_config_create_child_status: ", status);
    }
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
            if VERBOSE_SETUP_LOGS {
                write_status("tls_config_protocol_status: ", EFI_SUCCESS);
            }
            let mut ca_size = 0usize;
            let get_status = unsafe {
                ((*tls_config).get_data)(
                    tls_config,
                    EFI_TLS_CONFIG_DATA_TYPE_CA_CERTIFICATE,
                    null_mut(),
                    &mut ca_size,
                )
            };
            if VERBOSE_SETUP_LOGS {
                write_status("tls_config_get_ca_status: ", get_status);
                write_ascii("tls_config_get_ca_size: ");
                write_dec(ca_size as u64);
                write_ascii("\r\n");
            }

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
            if VERBOSE_SETUP_LOGS {
                write_status("tls_config_get_ca_after_status: ", get_after_status);
                write_ascii("tls_config_get_ca_after_size: ");
                write_dec(ca_size as u64);
                write_ascii("\r\n");
            }
        }
        Err(status) => {
            if VERBOSE_SETUP_LOGS {
                write_status("tls_config_protocol_status: ", status);
            }
        }
    }

    let destroy_status = unsafe { ((*binding).destroy_child)(binding, child) };
    if VERBOSE_SETUP_LOGS {
        write_status("tls_config_destroy_child_status: ", destroy_status);
    }
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
            if VERBOSE_SETUP_LOGS {
                write_ascii(label);
                write_ascii("_protocol_status: 0x");
                write_hex64(EFI_SUCCESS.0);
                write_ascii("\r\n");
            }

            let mut ca_size = 0usize;
            let get_status = unsafe {
                ((*tls_config).get_data)(
                    tls_config,
                    EFI_TLS_CONFIG_DATA_TYPE_CA_CERTIFICATE,
                    null_mut(),
                    &mut ca_size,
                )
            };
            if VERBOSE_SETUP_LOGS {
                write_ascii(label);
                write_ascii("_get_ca_status: 0x");
                write_hex64(get_status.0);
                write_ascii("\r\n");
                write_ascii(label);
                write_ascii("_get_ca_size: ");
                write_dec(ca_size as u64);
                write_ascii("\r\n");
            }

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
            if VERBOSE_SETUP_LOGS {
                write_ascii(label);
                write_ascii("_get_ca_after_status: 0x");
                write_hex64(get_after_status.0);
                write_ascii("\r\n");
                write_ascii(label);
                write_ascii("_get_ca_after_size: ");
                write_dec(ca_size as u64);
                write_ascii("\r\n");
            }
        }
        Err(status) => {
            if VERBOSE_SETUP_LOGS {
                write_ascii(label);
                write_ascii("_protocol_status: 0x");
                write_hex64(status.0);
                write_ascii("\r\n");
            }
        }
    }
}

fn tls_set_u32(tls: *mut EfiTlsProtocol, data_type: u32, value: &mut u32) -> EfiStatus {
    unsafe {
        ((*tls).set_session_data)(
            tls,
            data_type,
            value as *mut u32 as *mut c_void,
            core::mem::size_of::<u32>(),
        )
    }
}

fn build_tls_clienthello_on_child(
    bs: *mut EfiBootServices,
    child: EfiHandle,
    out: &mut [u8],
    out_len: &mut usize,
    label: &str,
) -> EfiStatus {
    configure_tls_ca_on_handle(bs, child, label);

    let tls = match open_protocol::<EfiTlsProtocol>(bs, child, &EFI_TLS_PROTOCOL_GUID) {
        Ok(tls) => {
            write_status("tls_probe_protocol_status: ", EFI_SUCCESS);
            tls
        }
        Err(status) => {
            write_status("tls_probe_protocol_status: ", status);
            return status;
        }
    };

    let mut connection_end = EFI_TLS_CONNECTION_END_CLIENT;
    let status = tls_set_u32(
        tls,
        EFI_TLS_SESSION_DATA_TYPE_CONNECTION_END,
        &mut connection_end,
    );
    write_status("tls_probe_set_connection_end_status: ", status);

    let mut verify = EFI_TLS_VERIFY_NONE;
    let status = tls_set_u32(tls, EFI_TLS_SESSION_DATA_TYPE_VERIFY_METHOD, &mut verify);
    write_status("tls_probe_set_verify_status: ", status);

    let mut version = EfiTlsVersion { major: 3, minor: 3 };
    let status = unsafe {
        ((*tls).set_session_data)(
            tls,
            EFI_TLS_SESSION_DATA_TYPE_VERSION,
            &mut version as *mut EfiTlsVersion as *mut c_void,
            core::mem::size_of::<EfiTlsVersion>(),
        )
    };
    write_status("tls_probe_set_version_status: ", status);

    *out_len = out.len();
    unsafe { ((*tls).build_response_packet)(tls, null_mut(), 0, out.as_mut_ptr(), out_len) }
}

fn run_tls_clienthello_probe(bs: *mut EfiBootServices) -> EfiStatus {
    write_ascii("tls_probe_start\r\n");
    let mut service_count = 0usize;
    let mut service_handles = null_mut();
    let status = locate_protocol_handles(
        bs,
        &EFI_TLS_SERVICE_BINDING_PROTOCOL_GUID,
        &mut service_count,
        &mut service_handles,
    );
    write_status("tls_probe_service_status: ", status);
    write_ascii("tls_probe_service_handle_count: ");
    write_dec(service_count as u64);
    write_ascii("\r\n");
    if status.is_error() || service_count == 0 || service_handles.is_null() {
        return status;
    }

    let service_handle = unsafe { *service_handles };
    let binding = match open_protocol::<EfiServiceBindingProtocol>(
        bs,
        service_handle,
        &EFI_TLS_SERVICE_BINDING_PROTOCOL_GUID,
    ) {
        Ok(binding) => binding,
        Err(status) => {
            write_status("tls_probe_binding_open_status: ", status);
            unsafe {
                ((*bs).free_pool)(service_handles as *mut c_void);
            }
            return status;
        }
    };

    let mut child = null_mut();
    let status = unsafe { ((*binding).create_child)(binding, &mut child) };
    write_status("tls_probe_create_child_status: ", status);
    if status.is_error() || child.is_null() {
        unsafe {
            ((*bs).free_pool)(service_handles as *mut c_void);
        }
        return if status.is_error() {
            status
        } else {
            EFI_UNSUPPORTED
        };
    }

    let mut out = [0u8; 2048];
    let mut out_len = 0usize;
    let status =
        build_tls_clienthello_on_child(bs, child, &mut out, &mut out_len, "tls_probe_config");
    write_status("tls_probe_build_clienthello_status: ", status);
    write_ascii("tls_probe_clienthello_len: ");
    write_dec(out_len as u64);
    write_ascii("\r\n");
    if out_len >= 5 {
        write_ascii("tls_probe_clienthello_first5: ");
        let mut i = 0usize;
        while i < 5 {
            write_ascii("0x");
            write_hex64(out[i] as u64);
            if i + 1 < 5 {
                write_ascii(",");
            }
            i += 1;
        }
        write_ascii("\r\n");
    }

    unsafe {
        ((*binding).destroy_child)(binding, child);
        ((*bs).free_pool)(service_handles as *mut c_void);
    }
    status
}
