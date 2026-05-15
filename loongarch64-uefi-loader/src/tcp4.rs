const KERNEL_TRACE_RECORDS: bool = false;

fn poll_tcp4(tcp4: *mut EfiTcp4Protocol, token: *const EfiTcp4CompletionToken) -> EfiStatus {
    let mut i = 0usize;
    while i < TCP4_POLL_LIMIT {
        let status = unsafe { read_volatile(core::ptr::addr_of!((*token).status)) };
        if status != EFI_NOT_READY {
            return status;
        }
        unsafe {
            ((*tcp4).poll)(tcp4);
        }
        i += 1;
    }
    unsafe { read_volatile(core::ptr::addr_of!((*token).status)) }
}

fn poll_tcp4_limit(
    tcp4: *mut EfiTcp4Protocol,
    token: *const EfiTcp4CompletionToken,
    limit: usize,
) -> EfiStatus {
    let mut i = 0usize;
    while i < limit {
        let status = unsafe { read_volatile(core::ptr::addr_of!((*token).status)) };
        if status != EFI_NOT_READY {
            return status;
        }
        unsafe {
            ((*tcp4).poll)(tcp4);
        }
        i += 1;
    }
    unsafe { read_volatile(core::ptr::addr_of!((*token).status)) }
}

fn tcp4_connect_probe(
    bs: *mut EfiBootServices,
    binding: *mut EfiServiceBindingProtocol,
    remote: [u8; 4],
    port: u16,
    label: &str,
) -> EfiStatus {
    let mut child = null_mut();
    let status = unsafe { ((*binding).create_child)(binding, &mut child) };
    write_prefixed_status(label, "_create_child_status", status);
    if status.is_error() || child.is_null() {
        return if status.is_error() {
            status
        } else {
            EFI_UNSUPPORTED
        };
    }

    let tcp4 = match open_protocol::<EfiTcp4Protocol>(bs, child, &EFI_TCP4_PROTOCOL_GUID) {
        Ok(tcp4) => {
            write_prefixed_status(label, "_protocol_status", EFI_SUCCESS);
            tcp4
        }
        Err(status) => {
            write_prefixed_status(label, "_protocol_status", status);
            unsafe {
                ((*binding).destroy_child)(binding, child);
            }
            return status;
        }
    };

    let mut config = EfiTcp4ConfigData {
        type_of_service: 0,
        time_to_live: 64,
        access_point: EfiTcp4AccessPoint {
            use_default_address: 1,
            station_address: [0; 4],
            subnet_mask: [0; 4],
            station_port: 0,
            remote_address: remote,
            remote_port: port,
            active_flag: 1,
        },
        control_option: EfiTcp4Option {
            receive_buffer_size: 0,
            send_buffer_size: 0,
            max_syn_back_log: 0,
            connection_timeout: 0,
            data_retries: 0,
            fin_timeout: 0,
            time_wait_timeout: 0,
            keep_alive_probes: 0,
            keep_alive_time: 0,
            keep_alive_interval: 0,
            enable_nagle: 0,
            enable_time_stamp: 0,
            enable_window_scaling: 0,
            enable_selective_ack: 0,
            enable_path_mtu_discovery: 0,
        },
    };

    let status = unsafe { ((*tcp4).configure)(tcp4, &mut config) };
    write_prefixed_status(label, "_configure_status", status);
    if status.is_error() {
        unsafe {
            ((*binding).destroy_child)(binding, child);
        }
        return status;
    }

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
    write_prefixed_status(label, "_connect_event_status", status);
    if status.is_error() {
        unsafe {
            ((*tcp4).configure)(tcp4, null_mut());
            ((*binding).destroy_child)(binding, child);
        }
        return status;
    }

    let mut token = EfiTcp4ConnectionToken {
        completion_token: EfiTcp4CompletionToken {
            event,
            status: EFI_NOT_READY,
        },
    };
    let submit_status = unsafe { ((*tcp4).connect)(tcp4, &mut token) };
    write_prefixed_status(label, "_connect_submit_status", submit_status);

    let completion = if submit_status.is_error() {
        submit_status
    } else {
        poll_tcp4(tcp4, &token.completion_token)
    };
    write_prefixed_status(label, "_connect_completion", completion);
    write_prefixed_status(
        label,
        "_connect_token_status",
        token.completion_token.status,
    );

    let mut close_event = null_mut();
    let close_event_status = unsafe {
        ((*bs).create_event)(
            EVT_NOTIFY_SIGNAL,
            TPL_CALLBACK,
            Some(noop_event),
            null_mut(),
            &mut close_event,
        )
    };
    write_prefixed_status(label, "_close_event_status", close_event_status);
    if !close_event_status.is_error() {
        let mut close_token = EfiTcp4CloseToken {
            completion_token: EfiTcp4CompletionToken {
                event: close_event,
                status: EFI_NOT_READY,
            },
            abort_on_close: 1,
        };
        let close_submit_status = unsafe { ((*tcp4).close)(tcp4, &mut close_token) };
        write_prefixed_status(label, "_close_submit_status", close_submit_status);
        if !close_submit_status.is_error() {
            let close_completion = poll_tcp4(tcp4, &close_token.completion_token);
            write_prefixed_status(label, "_close_completion", close_completion);
        }
        unsafe {
            ((*bs).close_event)(close_event);
        }
    }

    unsafe {
        ((*bs).close_event)(event);
        ((*tcp4).configure)(tcp4, null_mut());
        ((*binding).destroy_child)(binding, child);
    }

    completion
}

fn tcp4_transmit_once(
    bs: *mut EfiBootServices,
    tcp4: *mut EfiTcp4Protocol,
    bytes: &mut [u8],
    label: &str,
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
    write_prefixed_status(label, "_tx_event_status", status);
    if status.is_error() {
        return status;
    }

    let mut tx_data = EfiTcp4TransmitData {
        push: 1,
        urgent: 0,
        data_length: bytes.len() as u32,
        fragment_count: 1,
        fragment_table: [EfiTcp4FragmentData {
            fragment_length: bytes.len() as u32,
            fragment_buffer: bytes.as_mut_ptr() as *mut c_void,
        }],
    };
    let mut token = EfiTcp4IoToken {
        completion_token: EfiTcp4CompletionToken {
            event,
            status: EFI_NOT_READY,
        },
        packet: EfiTcp4IoPacket {
            tx_data: &mut tx_data,
        },
    };

    let submit_status = unsafe { ((*tcp4).transmit)(tcp4, &mut token) };
    write_prefixed_status(label, "_tx_submit_status", submit_status);
    let completion = if submit_status.is_error() {
        submit_status
    } else {
        poll_tcp4(tcp4, &token.completion_token)
    };
    write_prefixed_status(label, "_tx_completion", completion);
    write_prefixed_status(label, "_tx_token_status", token.completion_token.status);
    unsafe {
        ((*bs).close_event)(event);
    }
    completion
}

fn tcp4_receive_once(
    bs: *mut EfiBootServices,
    tcp4: *mut EfiTcp4Protocol,
    out: &mut [u8],
    received_len: &mut usize,
    label: &str,
) -> EfiStatus {
    *received_len = 0;
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
    write_prefixed_status(label, "_rx_event_status", status);
    if status.is_error() {
        return status;
    }

    let mut rx_data = EfiTcp4ReceiveData {
        urgent_flag: 0,
        data_length: out.len() as u32,
        fragment_count: 1,
        fragment_table: [EfiTcp4FragmentData {
            fragment_length: out.len() as u32,
            fragment_buffer: out.as_mut_ptr() as *mut c_void,
        }],
    };
    let mut token = EfiTcp4IoToken {
        completion_token: EfiTcp4CompletionToken {
            event,
            status: EFI_NOT_READY,
        },
        packet: EfiTcp4IoPacket {
            rx_data: &mut rx_data,
        },
    };

    let submit_status = unsafe { ((*tcp4).receive)(tcp4, &mut token) };
    write_prefixed_status(label, "_rx_submit_status", submit_status);
    let completion = if submit_status.is_error() {
        submit_status
    } else {
        poll_tcp4(tcp4, &token.completion_token)
    };
    write_prefixed_status(label, "_rx_completion", completion);
    write_prefixed_status(label, "_rx_token_status", token.completion_token.status);
    if completion.is_error() {
        *received_len = 0;
    } else {
        *received_len = rx_data.data_length as usize;
    }
    write_ascii(label);
    write_ascii("_rx_len: ");
    write_dec(*received_len as u64);
    write_ascii("\r\n");
    if *received_len >= 5 {
        write_ascii(label);
        write_ascii("_rx_first5: ");
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
        ((*bs).close_event)(event);
    }
    completion
}

fn print_tcp4_first5(label: &str, suffix: &str, bytes: &[u8], len: usize) {
    if len < 5 {
        return;
    }
    write_ascii(label);
    write_ascii(suffix);
    let mut i = 0usize;
    while i < 5 {
        write_ascii("0x");
        write_hex64(bytes[i] as u64);
        if i + 1 < 5 {
            write_ascii(",");
        }
        i += 1;
    }
    write_ascii("\r\n");
}

fn tls_session_state(tls: *mut EfiTlsProtocol, label: &str) -> u32 {
    let mut state = 0u32;
    let mut state_len = core::mem::size_of::<u32>();
    let status = unsafe {
        ((*tls).get_session_data)(
            tls,
            EFI_TLS_SESSION_DATA_TYPE_SESSION_STATE,
            &mut state as *mut u32 as *mut c_void,
            &mut state_len,
        )
    };
    write_ascii(label);
    write_ascii("_state_status: ");
    write_ascii("0x");
    write_hex64(status.0);
    write_ascii("\r\n");
    write_ascii(label);
    write_ascii("_state: ");
    write_dec(state as u64);
    write_ascii("\r\n");
    state
}

fn print_ascii_prefix(label: &str, bytes: &[u8], len: usize) {
    let mut count = len;
    if count > 64 {
        count = 64;
    }
    write_ascii(label);
    write_ascii("_ascii_prefix: ");
    let mut i = 0usize;
    while i < count {
        let b = bytes[i];
        if b == b'\r' {
            write_ascii("\\r");
        } else if b == b'\n' {
            write_ascii("\\n");
        } else if b >= 0x20 && b < 0x7f {
            write_bytes(&[b]);
        } else {
            write_ascii(".");
        }
        i += 1;
    }
    write_ascii("\r\n");
}

fn find_http_body_offset(bytes: &[u8], len: usize) -> Option<usize> {
    let mut i = 0usize;
    while i + 3 < len {
        if bytes[i] == b'\r'
            && bytes[i + 1] == b'\n'
            && bytes[i + 2] == b'\r'
            && bytes[i + 3] == b'\n'
        {
            return Some(i + 4);
        }
        i += 1;
    }
    None
}

fn find_http_status_offset(bytes: &[u8], len: usize) -> Option<usize> {
    let mut i = 0usize;
    while i + 11 < len {
        if bytes[i] == b'H'
            && bytes[i + 1] == b'T'
            && bytes[i + 2] == b'T'
            && bytes[i + 3] == b'P'
            && bytes[i + 4] == b'/'
        {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn http_response_status_is_200(bytes: &[u8], len: usize) -> bool {
    match find_http_status_offset(bytes, len) {
        Some(offset) => {
            offset + 11 < len
                && bytes[offset + 8] == b' '
                && bytes[offset + 9] == b'2'
                && bytes[offset + 10] == b'0'
                && bytes[offset + 11] == b'0'
        }
        None => false,
    }
}

fn https_path_from_url<'a>(url: &'a [u8]) -> Result<&'a [u8], EfiStatus> {
    const HTTPS_PREFIX: &[u8] = b"https://";
    if !starts_with(url, HTTPS_PREFIX) {
        return Err(EFI_UNSUPPORTED);
    }
    let mut i = HTTPS_PREFIX.len();
    while i < url.len() {
        if url[i] == b'/' {
            return Ok(&url[i..]);
        }
        i += 1;
    }
    Err(EFI_UNSUPPORTED)
}

fn build_http_get_request(path: &[u8], connection: &[u8], out: &mut [u8]) -> Result<usize, EfiStatus> {
    const PREFIX: &[u8] = b"GET ";
    const VERSION_HOST: &[u8] = b" HTTP/1.1\r\nHost: 10.3.10.229\r\nConnection: ";
    const SUFFIX: &[u8] = b"\r\n\r\n";
    let needed = PREFIX.len() + path.len() + VERSION_HOST.len() + connection.len() + SUFFIX.len();
    if needed > out.len() {
        return Err(EFI_BUFFER_TOO_SMALL);
    }
    let mut n = 0usize;
    let mut i = 0usize;
    while i < PREFIX.len() {
        out[n] = PREFIX[i];
        n += 1;
        i += 1;
    }
    i = 0;
    while i < path.len() {
        out[n] = path[i];
        n += 1;
        i += 1;
    }
    i = 0;
    while i < VERSION_HOST.len() {
        out[n] = VERSION_HOST[i];
        n += 1;
        i += 1;
    }
    i = 0;
    while i < connection.len() {
        out[n] = connection[i];
        n += 1;
        i += 1;
    }
    i = 0;
    while i < SUFFIX.len() {
        out[n] = SUFFIX[i];
        n += 1;
        i += 1;
    }
    Ok(n)
}

fn build_tls_plaintext_record(out: &mut [u8], payload: &[u8]) -> usize {
    if out.len() < payload.len() + 5 || payload.len() > 0xffff {
        return 0;
    }
    out[0] = 23;
    out[1] = 3;
    out[2] = 3;
    // UEFI TLS ProcessPacket(EfiTlsEncrypt) consumes TLS_RECORD_HEADER as a
    // firmware struct, so the plaintext input Length field is host-endian.
    out[3] = (payload.len() & 0xff) as u8;
    out[4] = ((payload.len() >> 8) & 0xff) as u8;
    let mut i = 0usize;
    while i < payload.len() {
        out[5 + i] = payload[i];
        i += 1;
    }
    payload.len() + 5
}

fn tls_record_total_len(bytes: &[u8], len: usize) -> usize {
    if len < 5 {
        return len;
    }
    let payload_len = ((bytes[3] as usize) << 8) | bytes[4] as usize;
    let total_len = payload_len + 5;
    if total_len <= len {
        total_len
    } else {
        len
    }
}

fn tls_record_declared_len(bytes: &[u8], len: usize) -> Option<usize> {
    if len < 5 {
        return None;
    }
    if bytes[0] != 23 || bytes[1] != 3 || bytes[2] != 3 {
        return None;
    }
    Some((((bytes[3] as usize) << 8) | bytes[4] as usize) + 5)
}

fn find_tls_app_record_start(bytes: &[u8], len: usize) -> Option<usize> {
    let mut i = 0usize;
    while i + 4 < len {
        if bytes[i] == 23 && bytes[i + 1] == 3 && bytes[i + 2] == 3 {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn tcp4_receive_kernel_once(
    bs: *mut EfiBootServices,
    tcp4: *mut EfiTcp4Protocol,
    out: &mut [u8],
    received_len: &mut usize,
    verbose: bool,
) -> EfiStatus {
    *received_len = 0;
    let mut last_status = EFI_NOT_READY;
    let mut attempt = 0usize;
    while attempt < KERNEL_TCP4_RX_RETRIES {
        if verbose {
            write_ascii("tcp4_tls_probe_kernel_rx_attempt: ");
            write_dec(attempt as u64);
            write_ascii("\r\n");
        }

        let mut event = null_mut();
        let event_status = unsafe {
            ((*bs).create_event)(
                EVT_NOTIFY_SIGNAL,
                TPL_CALLBACK,
                Some(noop_event),
                null_mut(),
                &mut event,
            )
        };
        if verbose || event_status.is_error() {
            write_status("tcp4_tls_probe_kernel_rx_event_status: ", event_status);
        }
        if event_status.is_error() {
            return event_status;
        }

        let mut rx_data = EfiTcp4ReceiveData {
            urgent_flag: 0,
            data_length: out.len() as u32,
            fragment_count: 1,
            fragment_table: [EfiTcp4FragmentData {
                fragment_length: out.len() as u32,
                fragment_buffer: out.as_mut_ptr() as *mut c_void,
            }],
        };
        let mut token = EfiTcp4IoToken {
            completion_token: EfiTcp4CompletionToken {
                event,
                status: EFI_NOT_READY,
            },
            packet: EfiTcp4IoPacket {
                rx_data: &mut rx_data,
            },
        };

        let submit_status = unsafe { ((*tcp4).receive)(tcp4, &mut token) };
        if verbose || submit_status.is_error() {
            write_status("tcp4_tls_probe_kernel_rx_submit_status: ", submit_status);
        }
        let completion = if submit_status.is_error() {
            submit_status
        } else {
            unsafe {
                if let Some(stall) = (*bs).stall {
                    stall(1_000_000);
                }
            }
            poll_tcp4_limit(tcp4, &token.completion_token, KERNEL_TCP4_POLL_LIMIT)
        };
        if verbose || completion.is_error() {
            write_status("tcp4_tls_probe_kernel_rx_completion: ", completion);
            write_status(
                "tcp4_tls_probe_kernel_rx_token_status: ",
                token.completion_token.status,
            );
        }

        if completion.is_error() {
            *received_len = 0;
            last_status = completion;
        } else {
            *received_len = rx_data.data_length as usize;
            if verbose {
                write_ascii("tcp4_tls_probe_kernel_rx_len: ");
                write_dec(*received_len as u64);
                write_ascii("\r\n");
                print_tcp4_first5(
                    "tcp4_tls_probe_kernel_rx",
                    "_first5: ",
                    out,
                    *received_len,
                );
            }
            unsafe {
                ((*bs).close_event)(event);
            }
            return completion;
        }

        unsafe {
            ((*bs).close_event)(event);
        }
        unsafe {
            if let Some(stall) = (*bs).stall {
                stall(200_000);
            }
        }
        attempt += 1;
    }
    last_status
}

fn https_send_get_with_pre_receive(
    bs: *mut EfiBootServices,
    tcp4: *mut EfiTcp4Protocol,
    tls: *mut EfiTlsProtocol,
    request: &[u8],
    encrypted_rx: &mut [u8],
    encrypted_rx_len: &mut usize,
    label: &str,
) -> EfiStatus {
    let mut plain_record = [0u8; 1536];
    let plain_record_len = build_tls_plaintext_record(&mut plain_record, request);
    write_ascii(label);
    write_ascii("_plain_record_len: ");
    write_dec(plain_record_len as u64);
    write_ascii("\r\n");
    if plain_record_len == 0 {
        return EFI_BUFFER_TOO_SMALL;
    }

    let (encrypt_status, encrypted_len, encrypted_ptr) =
        tls_process_single_fragment(tls, &mut plain_record, plain_record_len, EFI_TLS_ENCRYPT, label);
    if encrypt_status.is_error() || encrypted_len == 0 {
        return encrypt_status;
    }

    let mut encrypted = [0u8; 2048];
    let encrypted_len = copy_from_ptr(&mut encrypted, encrypted_ptr, encrypted_len);
    let encrypted_wire_len = tls_record_total_len(&encrypted, encrypted_len);
    write_ascii(label);
    write_ascii("_wire_len: ");
    write_dec(encrypted_wire_len as u64);
    write_ascii("\r\n");

    tcp4_transmit_with_pre_receive(
        bs,
        tcp4,
        &mut encrypted[..encrypted_wire_len],
        encrypted_rx,
        encrypted_rx_len,
        label,
    )
}

fn https_get_manifest_probe(
    bs: *mut EfiBootServices,
    tcp4: *mut EfiTcp4Protocol,
    tls: *mut EfiTlsProtocol,
    rx_status: &mut EfiStatus,
) -> Option<Manifest> {
    if rx_status.is_error() {
        return None;
    }

    let final_state = tls_session_state(tls, "tcp4_tls_probe_tls_final");
    if final_state != EFI_TLS_SESSION_DATA_TRANSFERRING {
        return None;
    }

    let mut http_get = [0u8; 512];
    let http_get_len = match build_http_get_request(
        b"/boot/boards/loongchip-httpboot-smoke/current/manifest.json",
        b"keep-alive",
        &mut http_get,
    ) {
        Ok(len) => len,
        Err(status) => {
            *rx_status = status;
            return None;
        }
    };

    let mut encrypted_rx = [0u8; TLS_RX_MAX];
    let mut encrypted_rx_len = 0usize;
    *rx_status = https_send_get_with_pre_receive(
        bs,
        tcp4,
        tls,
        &http_get[..http_get_len],
        &mut encrypted_rx,
        &mut encrypted_rx_len,
        "tcp4_tls_probe_app",
    );
    if rx_status.is_error() {
        return None;
    }

    let mut http_plain = [0u8; MANIFEST_MAX + 1024];
    let mut http_len = 0usize;
    let mut loop_count = 0usize;
    loop {
        let (decrypt_status, decrypted_len, decrypted_ptr) = tls_process_single_fragment(
            tls,
            &mut encrypted_rx,
            encrypted_rx_len,
            EFI_TLS_DECRYPT,
            "tcp4_tls_probe_app_decrypt",
        );
        *rx_status = decrypt_status;
        if rx_status.is_error() {
            return None;
        }
        if decrypted_len > 0 {
            let mut decrypted = [0u8; 1024];
            let decrypted_len = copy_from_ptr(&mut decrypted, decrypted_ptr, decrypted_len);
            if loop_count == 0 {
                print_ascii_prefix("tcp4_tls_probe_app_decrypt", &decrypted, decrypted_len);
            }
            if http_len + decrypted_len > http_plain.len() {
                *rx_status = EFI_BUFFER_TOO_SMALL;
                return None;
            }
            let mut i = 0usize;
            while i < decrypted_len {
                http_plain[http_len + i] = decrypted[i];
                i += 1;
            }
            http_len += decrypted_len;
        }

        if let Some(body_offset) = find_http_body_offset(&http_plain, http_len) {
            let body_len = http_len - body_offset;
            write_ascii("tcp4_tls_probe_manifest_http_len: ");
            write_dec(http_len as u64);
            write_ascii("\r\n");
            write_ascii("tcp4_tls_probe_manifest_body_len: ");
            write_dec(body_len as u64);
            write_ascii("\r\n");
            if body_len > 0 {
                match parse_manifest(&http_plain[body_offset..http_len]) {
                    Ok(manifest) => {
                        write_ascii("tcp4_tls_probe_manifest_arch: ");
                        write_bytes(&manifest.arch[..manifest.arch_len]);
                        write_ascii("\r\n");
                        write_ascii("tcp4_tls_probe_manifest_kernel_url: ");
                        write_bytes(&manifest.kernel_url[..manifest.kernel_url_len]);
                        write_ascii("\r\n");
                        write_ascii("tcp4_tls_probe_manifest_kernel_size: ");
                        write_dec(manifest.kernel_size);
                        write_ascii("\r\n");
                        write_ascii("tcp4_tls_probe_manifest_entry_point: 0x");
                        write_hex64(manifest.entry_point);
                        write_ascii("\r\n");
                        return Some(manifest);
                    }
                    Err(_) => {
                        write_ascii("tcp4_tls_probe_manifest_parse_wait_more\r\n");
                    }
                }
            } else {
                write_ascii("tcp4_tls_probe_manifest_wait_body\r\n");
            }
        }

        loop_count += 1;
        if loop_count >= 4 {
            *rx_status = EFI_NOT_READY;
            return None;
        }
        *rx_status = tcp4_receive_once(
            bs,
            tcp4,
            &mut encrypted_rx,
            &mut encrypted_rx_len,
            "tcp4_tls_probe_app_more",
        );
        if rx_status.is_error() {
            return None;
        }
    }
}

fn https_download_kernel_probe(
    bs: *mut EfiBootServices,
    tcp4: *mut EfiTcp4Protocol,
    tls: *mut EfiTlsProtocol,
    manifest: &Manifest,
) -> EfiStatus {
    if manifest.kernel_size == 0 || manifest.kernel_size > MAX_KERNEL_SIZE {
        return EFI_UNSUPPORTED;
    }
    if (manifest.kernel_load_addr as usize % EFI_PAGE_SIZE) != 0 {
        return EFI_UNSUPPORTED;
    }

    let kernel_path = match https_path_from_url(&manifest.kernel_url[..manifest.kernel_url_len]) {
        Ok(path) => path,
        Err(status) => return status,
    };
    write_ascii("tcp4_tls_probe_kernel_path: ");
    write_bytes(kernel_path);
    write_ascii("\r\n");

    let pages = page_count(manifest.kernel_size);
    let mut target = manifest.kernel_load_addr;
    let mut allocate_status = unsafe {
        ((*bs).allocate_pages)(EFI_ALLOCATE_ADDRESS, EFI_LOADER_DATA, pages, &mut target)
    };
    write_status("tcp4_tls_probe_kernel_allocate_pages_status: ", allocate_status);
    write_ascii("tcp4_tls_probe_kernel_target_addr: 0x");
    write_hex64(target);
    write_ascii("\r\n");
    if allocate_status.is_error() || target != manifest.kernel_load_addr {
        write_ascii("tcp4_tls_probe_kernel_staging_fallback: yes\r\n");
        target = 0;
        allocate_status = unsafe {
            ((*bs).allocate_pages)(EFI_ALLOCATE_ANY_PAGES, EFI_LOADER_DATA, pages, &mut target)
        };
        write_status(
            "tcp4_tls_probe_kernel_staging_allocate_pages_status: ",
            allocate_status,
        );
        write_ascii("tcp4_tls_probe_kernel_staging_addr: 0x");
        write_hex64(target);
        write_ascii("\r\n");
        if allocate_status.is_error() {
            return allocate_status;
        }
    }

    let mut http_get = [0u8; 1536];
    let http_get_len = match build_http_get_request(kernel_path, b"close", &mut http_get) {
        Ok(len) => len,
        Err(status) => {
            unsafe {
                ((*bs).free_pages)(target, pages);
            }
            return status;
        }
    };

    let mut encrypted_rx = [0u8; TLS_RX_MAX];
    let mut encrypted_rx_len = 0usize;
    let mut status = https_send_get_with_pre_receive(
        bs,
        tcp4,
        tls,
        &http_get[..http_get_len],
        &mut encrypted_rx,
        &mut encrypted_rx_len,
        "tcp4_tls_probe_kernel",
    );
    if status.is_error() {
        unsafe {
            ((*bs).free_pages)(target, pages);
        }
        return status;
    }

    let mut header = [0u8; 2048];
    let mut header_len = 0usize;
    let mut saw_header = false;
    let mut downloaded = 0u64;
    let mut checksum = 0u32;
    let mut next_progress = 256u64 * 1024;
    let mut loop_count = 0usize;
    let mut encrypted_rx_offset = 0usize;

    while downloaded < manifest.kernel_size {
        if encrypted_rx_offset >= encrypted_rx_len {
            loop_count += 1;
            if loop_count > 2048 {
                status = EFI_NOT_READY;
                break;
            }
            encrypted_rx_offset = 0;
            status = tcp4_receive_kernel_once(
                bs,
                tcp4,
                &mut encrypted_rx,
                &mut encrypted_rx_len,
                KERNEL_TRACE_RECORDS,
            );
            if status.is_error() {
                break;
            }
        }

        let available_len = encrypted_rx_len - encrypted_rx_offset;
        if tls_record_declared_len(&encrypted_rx[encrypted_rx_offset..], available_len).is_none() {
            match find_tls_app_record_start(&encrypted_rx[encrypted_rx_offset..], available_len) {
                Some(skip_len) if skip_len > 0 => {
                    write_ascii("tcp4_tls_probe_kernel_resync_skip_len: ");
                    write_dec(skip_len as u64);
                    write_ascii("\r\n");
                    encrypted_rx_offset += skip_len;
                    continue;
                }
                Some(_) => {}
                None => {
                    let carry_len = if available_len > 4 { 4 } else { available_len };
                    let mut i = 0usize;
                    while i < carry_len {
                        encrypted_rx[i] =
                            encrypted_rx[encrypted_rx_offset + available_len - carry_len + i];
                        i += 1;
                    }
                    encrypted_rx_offset = 0;
                    encrypted_rx_len = carry_len;
                    if KERNEL_TRACE_RECORDS {
                        write_ascii("tcp4_tls_probe_kernel_resync_carry_len: ");
                        write_dec(carry_len as u64);
                        write_ascii("\r\n");
                    }
                    let mut appended_len = 0usize;
                    status = tcp4_receive_kernel_once(
                        bs,
                        tcp4,
                        &mut encrypted_rx[encrypted_rx_len..],
                        &mut appended_len,
                        KERNEL_TRACE_RECORDS,
                    );
                    if status.is_error() {
                        break;
                    }
                    encrypted_rx_len += appended_len;
                    continue;
                }
            }
        }
        let current_record_len =
            match tls_record_declared_len(&encrypted_rx[encrypted_rx_offset..], available_len) {
                Some(record_len) => record_len,
                None => {
                    let carry_len = if available_len > 4 { 4 } else { available_len };
                    let mut i = 0usize;
                    while i < carry_len {
                        encrypted_rx[i] =
                            encrypted_rx[encrypted_rx_offset + available_len - carry_len + i];
                        i += 1;
                    }
                    encrypted_rx_offset = 0;
                    encrypted_rx_len = carry_len;
                    if KERNEL_TRACE_RECORDS {
                        write_ascii("tcp4_tls_probe_kernel_carry_len: ");
                        write_dec(carry_len as u64);
                        write_ascii("\r\n");
                    }
                    if encrypted_rx_len >= encrypted_rx.len() {
                        status = EFI_BUFFER_TOO_SMALL;
                        break;
                    }
                    let mut appended_len = 0usize;
                    status = tcp4_receive_kernel_once(
                        bs,
                        tcp4,
                        &mut encrypted_rx[encrypted_rx_len..],
                        &mut appended_len,
                        KERNEL_TRACE_RECORDS,
                    );
                    if status.is_error() {
                        break;
                    }
                    encrypted_rx_len += appended_len;
                    continue;
                }
            };
        if current_record_len > available_len {
            let carry_len = available_len;
            let mut i = 0usize;
            while i < carry_len {
                encrypted_rx[i] = encrypted_rx[encrypted_rx_offset + i];
                i += 1;
            }
            encrypted_rx_offset = 0;
            encrypted_rx_len = carry_len;
            if KERNEL_TRACE_RECORDS {
                write_ascii("tcp4_tls_probe_kernel_carry_len: ");
                write_dec(carry_len as u64);
                write_ascii("\r\n");
                write_ascii("tcp4_tls_probe_kernel_record_need_len: ");
                write_dec(current_record_len as u64);
                write_ascii("\r\n");
            }
            if encrypted_rx_len >= encrypted_rx.len() {
                status = EFI_BUFFER_TOO_SMALL;
                break;
            }
            let mut appended_len = 0usize;
            status = tcp4_receive_kernel_once(
                bs,
                tcp4,
                &mut encrypted_rx[encrypted_rx_len..],
                &mut appended_len,
                KERNEL_TRACE_RECORDS,
            );
            if status.is_error() {
                break;
            }
            encrypted_rx_len += appended_len;
            continue;
        }
        if KERNEL_TRACE_RECORDS {
            write_ascii("tcp4_tls_probe_kernel_record_len: ");
            write_dec(current_record_len as u64);
            write_ascii("\r\n");
        }

        let (decrypt_status, decrypted_len, decrypted_ptr) = tls_process_single_fragment_quiet(
            tls,
            &mut encrypted_rx[encrypted_rx_offset..encrypted_rx_offset + current_record_len],
            current_record_len,
            EFI_TLS_DECRYPT,
        );
        encrypted_rx_offset += current_record_len;
        status = decrypt_status;
        if status.is_error() {
            write_status("tcp4_tls_probe_kernel_decrypt_status: ", status);
            write_ascii("tcp4_tls_probe_kernel_decrypt_record_len: ");
            write_dec(current_record_len as u64);
            write_ascii("\r\n");
            break;
        }

        let mut offset = 0usize;
        let mut data_len = decrypted_len;
        if !saw_header {
            let mut i = 0usize;
            while i < decrypted_len && header_len < header.len() {
                let b = unsafe { read_volatile((decrypted_ptr as *const u8).add(i)) };
                header[header_len] = b;
                header_len += 1;
                i += 1;
                if let Some(body_offset) = find_http_body_offset(&header, header_len) {
                    write_ascii("tcp4_tls_probe_kernel_header_len: ");
                    write_dec(body_offset as u64);
                    write_ascii("\r\n");
                    if !http_response_status_is_200(&header, header_len) {
                        status = EFI_DEVICE_ERROR;
                    }
                    saw_header = true;
                    offset = i;
                    data_len = decrypted_len - offset;
                    break;
                }
            }
            if status.is_error() {
                break;
            }
            if !saw_header && header_len >= header.len() {
                status = EFI_BUFFER_TOO_SMALL;
                break;
            }
            if !saw_header {
                data_len = 0;
            }
        }

        if saw_header && data_len > 0 {
            let remaining = (manifest.kernel_size - downloaded) as usize;
            let copy_len = if data_len > remaining { remaining } else { data_len };
            let src = unsafe { (decrypted_ptr as *const u8).add(offset) };
            let dst = (target + downloaded) as *mut u8;
            let mut i = 0usize;
            while i < copy_len {
                let b = unsafe { read_volatile(src.add(i)) };
                unsafe {
                    core::ptr::write_volatile(dst.add(i), b);
                }
                checksum = checksum.wrapping_add(b as u32);
                i += 1;
            }
            downloaded += copy_len as u64;
            if downloaded >= next_progress {
                write_ascii("tcp4_tls_probe_kernel_progress: ");
                write_dec(downloaded);
                write_ascii("\r\n");
                next_progress += 256u64 * 1024;
            }
        }

        if downloaded >= manifest.kernel_size {
            break;
        }
    }

    write_ascii("tcp4_tls_probe_kernel_downloaded_size: ");
    write_dec(downloaded);
    write_ascii("\r\n");
    write_ascii("tcp4_tls_probe_kernel_expected_size: ");
    write_dec(manifest.kernel_size);
    write_ascii("\r\n");
    write_ascii("tcp4_tls_probe_kernel_checksum32: 0x");
    write_hex64(checksum as u64);
    write_ascii("\r\n");

    if status.is_error() || downloaded != manifest.kernel_size {
        unsafe {
            ((*bs).free_pages)(target, pages);
        }
        return if status.is_error() {
            status
        } else {
            EFI_DEVICE_ERROR
        };
    }

    EFI_SUCCESS
}

fn tls_process_single_fragment(
    tls: *mut EfiTlsProtocol,
    bytes: &mut [u8],
    len: usize,
    process_type: u32,
    label: &str,
) -> (EfiStatus, usize, *mut c_void) {
    let mut fragment = EfiTlsFragmentData {
        fragment_length: len as u32,
        fragment_buffer: bytes.as_mut_ptr() as *mut c_void,
    };
    let mut fragment_ptr = &mut fragment as *mut EfiTlsFragmentData;
    let mut fragment_count = 1u32;
    let status = unsafe {
        ((*tls).process_packet)(tls, &mut fragment_ptr, &mut fragment_count, process_type)
    };
    write_prefixed_status(label, "_process_status", status);
    write_ascii(label);
    write_ascii("_process_fragment_count: ");
    write_dec(fragment_count as u64);
    write_ascii("\r\n");
    let fragment_len = if status.is_error() || fragment_ptr.is_null() {
        0
    } else {
        unsafe { (*fragment_ptr).fragment_length as usize }
    };
    write_ascii(label);
    write_ascii("_process_len: ");
    write_dec(fragment_len as u64);
    write_ascii("\r\n");
    let fragment_buffer = if status.is_error() || fragment_ptr.is_null() {
        null_mut()
    } else {
        unsafe { (*fragment_ptr).fragment_buffer }
    };
    (status, fragment_len, fragment_buffer)
}

fn tls_process_single_fragment_quiet(
    tls: *mut EfiTlsProtocol,
    bytes: &mut [u8],
    len: usize,
    process_type: u32,
) -> (EfiStatus, usize, *mut c_void) {
    let mut fragment = EfiTlsFragmentData {
        fragment_length: len as u32,
        fragment_buffer: bytes.as_mut_ptr() as *mut c_void,
    };
    let mut fragment_ptr = &mut fragment as *mut EfiTlsFragmentData;
    let mut fragment_count = 1u32;
    let status = unsafe {
        ((*tls).process_packet)(tls, &mut fragment_ptr, &mut fragment_count, process_type)
    };
    let fragment_len = if status.is_error() || fragment_ptr.is_null() {
        0
    } else {
        unsafe { (*fragment_ptr).fragment_length as usize }
    };
    let fragment_buffer = if status.is_error() || fragment_ptr.is_null() {
        null_mut()
    } else {
        unsafe { (*fragment_ptr).fragment_buffer }
    };
    (status, fragment_len, fragment_buffer)
}

fn copy_from_ptr(out: &mut [u8], src: *mut c_void, len: usize) -> usize {
    if src.is_null() {
        return 0;
    }
    let mut count = len;
    if count > out.len() {
        count = out.len();
    }
    let bytes = src as *const u8;
    let mut i = 0usize;
    while i < count {
        out[i] = unsafe { read_volatile(bytes.add(i)) };
        i += 1;
    }
    count
}

fn tcp4_transmit_with_pre_receive(
    bs: *mut EfiBootServices,
    tcp4: *mut EfiTcp4Protocol,
    bytes: &mut [u8],
    out: &mut [u8],
    received_len: &mut usize,
    label: &str,
) -> EfiStatus {
    *received_len = 0;

    let mut rx_event = null_mut();
    let rx_event_status = unsafe {
        ((*bs).create_event)(
            EVT_NOTIFY_SIGNAL,
            TPL_CALLBACK,
            Some(noop_event),
            null_mut(),
            &mut rx_event,
        )
    };
    write_prefixed_status(label, "_pre_rx_event_status", rx_event_status);

    let mut rx_data = EfiTcp4ReceiveData {
        urgent_flag: 0,
        data_length: out.len() as u32,
        fragment_count: 1,
        fragment_table: [EfiTcp4FragmentData {
            fragment_length: out.len() as u32,
            fragment_buffer: out.as_mut_ptr() as *mut c_void,
        }],
    };
    let mut rx_token = EfiTcp4IoToken {
        completion_token: EfiTcp4CompletionToken {
            event: rx_event,
            status: EFI_NOT_READY,
        },
        packet: EfiTcp4IoPacket {
            rx_data: &mut rx_data,
        },
    };
    let pre_rx_submit_status = if rx_event_status.is_error() {
        rx_event_status
    } else {
        unsafe { ((*tcp4).receive)(tcp4, &mut rx_token) }
    };
    write_prefixed_status(label, "_pre_rx_submit_status", pre_rx_submit_status);

    let tx_status = tcp4_transmit_once(bs, tcp4, bytes, label);
    if !tx_status.is_error() {
        unsafe {
            if let Some(stall) = (*bs).stall {
                stall(1_000_000);
            }
        }
        let mut warm_poll = EFI_NOT_READY;
        let mut i = 0usize;
        while i < 200 {
            warm_poll = unsafe { ((*tcp4).poll)(tcp4) };
            i += 1;
        }
        write_prefixed_status(label, "_post_tx_poll_status", warm_poll);
    }

    let rx_status = if tx_status.is_error() {
        tx_status
    } else if !pre_rx_submit_status.is_error() {
        let completion = poll_tcp4(tcp4, &rx_token.completion_token);
        write_prefixed_status(label, "_pre_rx_completion", completion);
        write_prefixed_status(label, "_pre_rx_token_status", rx_token.completion_token.status);
        if completion.is_error() {
            *received_len = 0;
        } else {
            *received_len = rx_data.data_length as usize;
        }
        write_ascii(label);
        write_ascii("_pre_rx_len: ");
        write_dec(*received_len as u64);
        write_ascii("\r\n");
        print_tcp4_first5(label, "_pre_rx_first5: ", out, *received_len);
        completion
    } else {
        tcp4_receive_once(bs, tcp4, out, received_len, label)
    };

    unsafe {
        if !rx_event.is_null() {
            ((*bs).close_event)(rx_event);
        }
    }

    rx_status
}

fn tcp4_tls_clienthello_probe(bs: *mut EfiBootServices) -> EfiStatus {
    write_ascii("tcp4_tls_probe_start\r\n");

    let mut tls_service_count = 0usize;
    let mut tls_service_handles = null_mut();
    let status = locate_protocol_handles(
        bs,
        &EFI_TLS_SERVICE_BINDING_PROTOCOL_GUID,
        &mut tls_service_count,
        &mut tls_service_handles,
    );
    write_status("tcp4_tls_probe_tls_service_status: ", status);
    write_ascii("tcp4_tls_probe_tls_service_handle_count: ");
    write_dec(tls_service_count as u64);
    write_ascii("\r\n");
    if status.is_error() || tls_service_count == 0 || tls_service_handles.is_null() {
        return status;
    }

    let tls_service_handle = unsafe { *tls_service_handles };
    let tls_binding = match open_protocol::<EfiServiceBindingProtocol>(
        bs,
        tls_service_handle,
        &EFI_TLS_SERVICE_BINDING_PROTOCOL_GUID,
    ) {
        Ok(binding) => binding,
        Err(status) => {
            write_status("tcp4_tls_probe_tls_binding_open_status: ", status);
            unsafe {
                ((*bs).free_pool)(tls_service_handles as *mut c_void);
            }
            return status;
        }
    };

    let mut tls_child = null_mut();
    let status = unsafe { ((*tls_binding).create_child)(tls_binding, &mut tls_child) };
    write_status("tcp4_tls_probe_tls_create_child_status: ", status);
    if status.is_error() || tls_child.is_null() {
        unsafe {
            ((*bs).free_pool)(tls_service_handles as *mut c_void);
        }
        return if status.is_error() {
            status
        } else {
            EFI_UNSUPPORTED
        };
    }

    let mut clienthello = [0u8; 2048];
    let mut clienthello_len = 0usize;
    let status = build_tls_clienthello_on_child(
        bs,
        tls_child,
        &mut clienthello,
        &mut clienthello_len,
        "tcp4_tls_probe_tls_config",
    );
    write_status("tcp4_tls_probe_build_clienthello_status: ", status);
    write_ascii("tcp4_tls_probe_clienthello_len: ");
    write_dec(clienthello_len as u64);
    write_ascii("\r\n");
    if status.is_error() || clienthello_len == 0 || clienthello_len > clienthello.len() {
        unsafe {
            ((*tls_binding).destroy_child)(tls_binding, tls_child);
            ((*bs).free_pool)(tls_service_handles as *mut c_void);
        }
        return status;
    }

    let mut tcp_service_count = 0usize;
    let mut tcp_service_handles = null_mut();
    let status = locate_protocol_handles(
        bs,
        &EFI_TCP4_SERVICE_BINDING_PROTOCOL_GUID,
        &mut tcp_service_count,
        &mut tcp_service_handles,
    );
    write_status("tcp4_tls_probe_tcp_service_status: ", status);
    write_ascii("tcp4_tls_probe_tcp_service_handle_count: ");
    write_dec(tcp_service_count as u64);
    write_ascii("\r\n");
    if status.is_error() || tcp_service_count == 0 || tcp_service_handles.is_null() {
        unsafe {
            ((*tls_binding).destroy_child)(tls_binding, tls_child);
            ((*bs).free_pool)(tls_service_handles as *mut c_void);
        }
        return status;
    }

    let tcp_service_handle = unsafe { *tcp_service_handles };
    let tcp_binding = match open_protocol::<EfiServiceBindingProtocol>(
        bs,
        tcp_service_handle,
        &EFI_TCP4_SERVICE_BINDING_PROTOCOL_GUID,
    ) {
        Ok(binding) => binding,
        Err(status) => {
            write_status("tcp4_tls_probe_tcp_binding_open_status: ", status);
            unsafe {
                ((*bs).free_pool)(tcp_service_handles as *mut c_void);
                ((*tls_binding).destroy_child)(tls_binding, tls_child);
                ((*bs).free_pool)(tls_service_handles as *mut c_void);
            }
            return status;
        }
    };

    let mut tcp_child = null_mut();
    let status = unsafe { ((*tcp_binding).create_child)(tcp_binding, &mut tcp_child) };
    write_status("tcp4_tls_probe_tcp_create_child_status: ", status);
    if status.is_error() || tcp_child.is_null() {
        unsafe {
            ((*bs).free_pool)(tcp_service_handles as *mut c_void);
            ((*tls_binding).destroy_child)(tls_binding, tls_child);
            ((*bs).free_pool)(tls_service_handles as *mut c_void);
        }
        return if status.is_error() {
            status
        } else {
            EFI_UNSUPPORTED
        };
    }

    let tcp4 = match open_protocol::<EfiTcp4Protocol>(bs, tcp_child, &EFI_TCP4_PROTOCOL_GUID) {
        Ok(tcp4) => {
            write_status("tcp4_tls_probe_tcp_protocol_status: ", EFI_SUCCESS);
            tcp4
        }
        Err(status) => {
            write_status("tcp4_tls_probe_tcp_protocol_status: ", status);
            unsafe {
                ((*tcp_binding).destroy_child)(tcp_binding, tcp_child);
                ((*bs).free_pool)(tcp_service_handles as *mut c_void);
                ((*tls_binding).destroy_child)(tls_binding, tls_child);
                ((*bs).free_pool)(tls_service_handles as *mut c_void);
            }
            return status;
        }
    };

    let mut config = EfiTcp4ConfigData {
        type_of_service: 0,
        time_to_live: 64,
        access_point: EfiTcp4AccessPoint {
            use_default_address: 1,
            station_address: [0; 4],
            subnet_mask: [0; 4],
            station_port: 0,
            remote_address: [10, 3, 10, 229],
            remote_port: 3443,
            active_flag: 1,
        },
        control_option: EfiTcp4Option {
            receive_buffer_size: 0,
            send_buffer_size: 0,
            max_syn_back_log: 0,
            connection_timeout: 0,
            data_retries: 0,
            fin_timeout: 0,
            time_wait_timeout: 0,
            keep_alive_probes: 0,
            keep_alive_time: 0,
            keep_alive_interval: 0,
            enable_nagle: 0,
            enable_time_stamp: 0,
            enable_window_scaling: 0,
            enable_selective_ack: 0,
            enable_path_mtu_discovery: 0,
        },
    };
    let status = unsafe { ((*tcp4).configure)(tcp4, &mut config) };
    write_status("tcp4_tls_probe_tcp_configure_status: ", status);
    if status.is_error() {
        unsafe {
            ((*tcp_binding).destroy_child)(tcp_binding, tcp_child);
            ((*bs).free_pool)(tcp_service_handles as *mut c_void);
            ((*tls_binding).destroy_child)(tls_binding, tls_child);
            ((*bs).free_pool)(tls_service_handles as *mut c_void);
        }
        return status;
    }

    let mut connect_event = null_mut();
    let status = unsafe {
        ((*bs).create_event)(
            EVT_NOTIFY_SIGNAL,
            TPL_CALLBACK,
            Some(noop_event),
            null_mut(),
            &mut connect_event,
        )
    };
    write_status("tcp4_tls_probe_connect_event_status: ", status);
    if status.is_error() {
        unsafe {
            ((*tcp4).configure)(tcp4, null_mut());
            ((*tcp_binding).destroy_child)(tcp_binding, tcp_child);
            ((*bs).free_pool)(tcp_service_handles as *mut c_void);
            ((*tls_binding).destroy_child)(tls_binding, tls_child);
            ((*bs).free_pool)(tls_service_handles as *mut c_void);
        }
        return status;
    }

    let mut connect_token = EfiTcp4ConnectionToken {
        completion_token: EfiTcp4CompletionToken {
            event: connect_event,
            status: EFI_NOT_READY,
        },
    };
    let submit_status = unsafe { ((*tcp4).connect)(tcp4, &mut connect_token) };
    write_status("tcp4_tls_probe_connect_submit_status: ", submit_status);
    let connect_status = if submit_status.is_error() {
        submit_status
    } else {
        poll_tcp4(tcp4, &connect_token.completion_token)
    };
    write_status("tcp4_tls_probe_connect_completion: ", connect_status);
    unsafe {
        ((*bs).close_event)(connect_event);
    }
    if connect_status.is_error() {
        unsafe {
            ((*tcp4).configure)(tcp4, null_mut());
            ((*tcp_binding).destroy_child)(tcp_binding, tcp_child);
            ((*bs).free_pool)(tcp_service_handles as *mut c_void);
            ((*tls_binding).destroy_child)(tls_binding, tls_child);
            ((*bs).free_pool)(tls_service_handles as *mut c_void);
        }
        return connect_status;
    }

    let tls = match open_protocol::<EfiTlsProtocol>(bs, tls_child, &EFI_TLS_PROTOCOL_GUID) {
        Ok(tls) => {
            write_status("tcp4_tls_probe_tls_protocol_reopen_status: ", EFI_SUCCESS);
            tls
        }
        Err(status) => {
            write_status("tcp4_tls_probe_tls_protocol_reopen_status: ", status);
            unsafe {
                ((*tcp4).configure)(tcp4, null_mut());
                ((*tcp_binding).destroy_child)(tcp_binding, tcp_child);
                ((*bs).free_pool)(tcp_service_handles as *mut c_void);
                ((*tls_binding).destroy_child)(tls_binding, tls_child);
                ((*bs).free_pool)(tls_service_handles as *mut c_void);
            }
            return status;
        }
    };
    tls_session_state(tls, "tcp4_tls_probe_tls_initial");

    let mut rx = [0u8; 8192];
    let mut rx_len = 0usize;
    let mut rx_status = tcp4_transmit_with_pre_receive(
        bs,
        tcp4,
        &mut clienthello[..clienthello_len],
        &mut rx,
        &mut rx_len,
        "tcp4_tls_probe",
    );

    let mut tls_out = [0u8; 8192];
    let mut round = 0usize;
    while !rx_status.is_error() && round < 4 {
        let label = match round {
            0 => "tcp4_tls_probe_hs0",
            1 => "tcp4_tls_probe_hs1",
            2 => "tcp4_tls_probe_hs2",
            _ => "tcp4_tls_probe_hs3",
        };

        tls_session_state(tls, label);
        let mut tls_out_len = tls_out.len();
        let build_status = unsafe {
            ((*tls).build_response_packet)(
                tls,
                rx.as_mut_ptr(),
                rx_len,
                tls_out.as_mut_ptr(),
                &mut tls_out_len,
            )
        };
        write_prefixed_status(label, "_build_response_status", build_status);
        write_ascii(label);
        write_ascii("_build_response_len: ");
        write_dec(tls_out_len as u64);
        write_ascii("\r\n");
        print_tcp4_first5(label, "_build_response_first5: ", &tls_out, tls_out_len);
        let state = tls_session_state(tls, label);
        if build_status.is_error() {
            rx_status = build_status;
            break;
        }
        if state == EFI_TLS_SESSION_DATA_TRANSFERRING {
            break;
        }
        if tls_out_len == 0 || tls_out_len > tls_out.len() {
            rx_status = EFI_NOT_READY;
            break;
        }
        rx_status = tcp4_transmit_with_pre_receive(
            bs,
            tcp4,
            &mut tls_out[..tls_out_len],
            &mut rx,
            &mut rx_len,
            label,
        );
        round += 1;
    }
    if let Some(manifest) = https_get_manifest_probe(bs, tcp4, tls, &mut rx_status) {
        write_ascii("tcp4_tls_probe_manifest_parse_status: ok\r\n");
        write_ascii("tcp4_tls_probe_next_kernel_size: ");
        write_dec(manifest.kernel_size);
        write_ascii("\r\n");
        rx_status = https_download_kernel_probe(bs, tcp4, tls, &manifest);
        write_status("tcp4_tls_probe_kernel_download_status: ", rx_status);
        if !rx_status.is_error() {
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
            }
        }
    }
    write_status("tcp4_tls_probe_result: ", rx_status);

    unsafe {
        ((*tcp4).configure)(tcp4, null_mut());
        ((*tcp_binding).destroy_child)(tcp_binding, tcp_child);
        ((*bs).free_pool)(tcp_service_handles as *mut c_void);
        ((*tls_binding).destroy_child)(tls_binding, tls_child);
        ((*bs).free_pool)(tls_service_handles as *mut c_void);
    }
    rx_status
}

fn run_tcp4_probe(bs: *mut EfiBootServices) -> EfiStatus {
    write_ascii("tcp4_probe_start\r\n");
    let mut service_count = 0usize;
    let mut service_handles = null_mut();
    let status = locate_protocol_handles(
        bs,
        &EFI_TCP4_SERVICE_BINDING_PROTOCOL_GUID,
        &mut service_count,
        &mut service_handles,
    );
    write_status("tcp4_probe_service_status: ", status);
    write_ascii("tcp4_probe_service_handle_count: ");
    write_dec(service_count as u64);
    write_ascii("\r\n");
    if status.is_error() || service_count == 0 || service_handles.is_null() {
        return status;
    }

    let remote = [10, 3, 10, 229];
    let mut result = EFI_NOT_FOUND;
    let mut i = 0usize;
    while i < service_count {
        let service_handle = unsafe { *service_handles.add(i) };
        let binding = match open_protocol::<EfiServiceBindingProtocol>(
            bs,
            service_handle,
            &EFI_TCP4_SERVICE_BINDING_PROTOCOL_GUID,
        ) {
            Ok(binding) => binding,
            Err(status) => {
                write_status("tcp4_probe_binding_open_status: ", status);
                i += 1;
                continue;
            }
        };

        result = tcp4_connect_probe(bs, binding, remote, 3443, "tcp4_probe_3443");
        if !result.is_error() {
            break;
        }

        result = tcp4_connect_probe(bs, binding, remote, 443, "tcp4_probe_443");
        if !result.is_error() {
            break;
        }

        i += 1;
    }

    unsafe {
        ((*bs).free_pool)(service_handles as *mut c_void);
    }
    write_status("tcp4_probe_result: ", result);
    result
}
