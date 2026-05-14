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
    if !VERBOSE_SETUP_LOGS {
        return;
    }
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
