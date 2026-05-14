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
    *received_len = rx_data.data_length as usize;
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

    let tx_status = tcp4_transmit_once(
        bs,
        tcp4,
        &mut clienthello[..clienthello_len],
        "tcp4_tls_probe",
    );
    let mut rx = [0u8; 2048];
    let mut rx_len = 0usize;
    let rx_status = if tx_status.is_error() {
        tx_status
    } else {
        tcp4_receive_once(bs, tcp4, &mut rx, &mut rx_len, "tcp4_tls_probe")
    };
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
