fn try_http_service_handle(
    image: EfiHandle,
    bs: *mut EfiBootServices,
    service_handle: EfiHandle,
    index: usize,
) -> EfiStatus {
    if VERBOSE_SETUP_LOGS {
        write_ascii("http_service_binding_try_index: ");
        write_dec(index as u64);
        write_ascii("\r\n");
    }

    let binding = match open_protocol::<EfiServiceBindingProtocol>(
        bs,
        service_handle,
        &EFI_HTTP_SERVICE_BINDING_PROTOCOL_GUID,
    ) {
        Ok(binding) => {
            if VERBOSE_SETUP_LOGS {
                write_status("http_service_binding_open_status: ", EFI_SUCCESS);
            }
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

fn warm_up_http_network_child(
    bs: *mut EfiBootServices,
    binding: *mut EfiServiceBindingProtocol,
    index: usize,
) -> EfiStatus {
    write_ascii("http_network_warmup_index: ");
    write_dec(index as u64);
    write_ascii("\r\n");

    let mut child = null_mut();
    let status = unsafe { ((*binding).create_child)(binding, &mut child) };
    write_status("http_network_warmup_create_child_status: ", status);
    if status.is_error() || child.is_null() {
        return if status.is_error() {
            status
        } else {
            EFI_UNSUPPORTED
        };
    }

    let http = match open_protocol::<EfiHttpProtocol>(bs, child, &EFI_HTTP_PROTOCOL_GUID) {
        Ok(http) => {
            write_status("http_network_warmup_protocol_status: ", EFI_SUCCESS);
            http
        }
        Err(status) => {
            write_status("http_network_warmup_protocol_status: ", status);
            unsafe {
                ((*binding).destroy_child)(binding, child);
            }
            return status;
        }
    };

    configure_tls_ca_on_handle(bs, child, "http_network_warmup_tls_config");
    let status = configure_http(http);
    write_status("http_network_warmup_configure_status: ", status);
    if !status.is_error() {
        warm_up_http(bs, http);
    }

    unsafe {
        ((*binding).destroy_child)(binding, child);
    }
    status
}

fn warm_up_http_network(bs: *mut EfiBootServices) -> EfiStatus {
    let mut service_count = 0usize;
    let mut service_handles = null_mut();
    let status = locate_protocol_handles(
        bs,
        &EFI_HTTP_SERVICE_BINDING_PROTOCOL_GUID,
        &mut service_count,
        &mut service_handles,
    );
    write_status("http_network_warmup_service_status: ", status);
    write_ascii("http_network_warmup_handle_count: ");
    write_dec(service_count as u64);
    write_ascii("\r\n");
    if status.is_error() || service_count == 0 || service_handles.is_null() {
        return status;
    }

    let mut result = EFI_NOT_FOUND;
    let mut i = 0usize;
    while i < service_count {
        let handle = unsafe { *service_handles.add(i) };
        let binding = match open_protocol::<EfiServiceBindingProtocol>(
            bs,
            handle,
            &EFI_HTTP_SERVICE_BINDING_PROTOCOL_GUID,
        ) {
            Ok(binding) => binding,
            Err(status) => {
                write_status("http_network_warmup_binding_status: ", status);
                i += 1;
                continue;
            }
        };
        result = warm_up_http_network_child(bs, binding, i);
        if !result.is_error() {
            break;
        }
        i += 1;
    }

    unsafe {
        ((*bs).free_pool)(service_handles as *mut c_void);
    }
    result
}

fn try_http_fallback(image: EfiHandle, bs: *mut EfiBootServices) -> EfiStatus {
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
        return status;
    }

    let mut result = EFI_NOT_FOUND;
    let mut i = 0usize;
    while i < service_count {
        let handle = unsafe { *service_handles.add(i) };
        result = try_http_service_handle(image, bs, handle, i);
        write_status("http_service_binding_try_status: ", result);
        if !result.is_error() {
            break;
        }
        i += 1;
    }
    unsafe {
        ((*bs).free_pool)(service_handles as *mut c_void);
    }
    result
}

fn stall_loader_retry(bs: *mut EfiBootServices) {
    unsafe {
        if let Some(stall) = (*bs).stall {
            stall(3_000_000);
        }
    }
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
    write_ascii("loader_build_id: ");
    write_ascii(OSTOOL_LOONGARCH64_LOADER_BUILD_ID);
    write_ascii("\r\n");
    write_ascii("log_mode: summary\r\n");

    let bs = unsafe {
        if system_table.is_null() {
            return EFI_SUCCESS;
        }
        (*system_table).boot_services
    };
    if bs.is_null() {
        return EFI_SUCCESS;
    }
    configure_serial_output(bs);

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
    run_tcp4_probe(bs);
    run_tls_clienthello_probe(bs);

    let mut attempt = 0usize;
    while attempt < 3 {
        write_ascii("loader_network_attempt: ");
        write_dec(attempt as u64);
        write_ascii("\r\n");

        let warmup_status = warm_up_http_network(bs);
        write_status("http_network_warmup_status: ", warmup_status);

        let tcp4_tls_status = tcp4_tls_clienthello_probe(image, bs);
        write_status("loader_tcp4_tls_attempt_status: ", tcp4_tls_status);
        if !tcp4_tls_status.is_error() {
            return EFI_SUCCESS;
        }

        let http_status = try_http_fallback(image, bs);
        write_status("loader_http_fallback_status: ", http_status);
        if !http_status.is_error() {
            return EFI_SUCCESS;
        }

        stall_loader_retry(bs);
        attempt += 1;
    }

    EFI_SUCCESS
}
