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
    if VERBOSE_SETUP_LOGS {
        write_status("http_post_configure_poll_status: ", last_poll);
    }
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
    if VERBOSE_SETUP_LOGS {
        write_ascii(label);
        write_ascii("_status: ");
        write_ascii("0x");
        write_hex64(status.0);
        write_ascii("\r\n");
        write_ascii(label);
        write_ascii("_handle_count: ");
        write_dec(count as u64);
        write_ascii("\r\n");
    }
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
