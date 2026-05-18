static mut MEMORY_MAP: [u8; MEMORY_MAP_MAX] = [0; MEMORY_MAP_MAX];
static mut KERNEL_BOOT_INFO: OstoolKernelBootInfo = OstoolKernelBootInfo {
    magic: OSTOOL_KERNEL_BOOT_INFO_MAGIC,
    version: 1,
    framebuffer_base: 0,
    framebuffer_size: 0,
    horizontal_resolution: 0,
    vertical_resolution: 0,
    pixels_per_scan_line: 0,
    pixel_format: 0xffff_ffff,
};

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

fn get_memory_map_key(bs: *mut EfiBootServices, map_key_out: &mut usize) -> EfiStatus {
    let mut map_size = MEMORY_MAP_MAX;
    let mut descriptor_size = 0usize;
    let mut descriptor_version = 0u32;
    let memory_map = core::ptr::addr_of_mut!(MEMORY_MAP) as *mut EfiMemoryDescriptor;
    unsafe {
        ((*bs).get_memory_map)(
            &mut map_size,
            memory_map,
            map_key_out,
            &mut descriptor_size,
            &mut descriptor_version,
        )
    }
}

fn exit_boot_services_with_fresh_map(image: EfiHandle, bs: *mut EfiBootServices) -> EfiStatus {
    let mut map_key = 0usize;
    let mut status = get_memory_map_key(bs, &mut map_key);
    if status.is_error() {
        return status;
    }
    status = unsafe { ((*bs).exit_boot_services)(image, map_key) };
    if status == EFI_INVALID_PARAMETER {
        status = get_memory_map_key(bs, &mut map_key);
        if status.is_error() {
            return status;
        }
        status = unsafe { ((*bs).exit_boot_services)(image, map_key) };
    }
    status
}

fn print_memory_descriptor(label: &str, descriptor: *const EfiMemoryDescriptor) {
    if descriptor.is_null() {
        write_ascii(label);
        write_ascii("_found: no\r\n");
        return;
    }

    let desc = unsafe { &*descriptor };
    write_ascii(label);
    write_ascii("_found: yes\r\n");
    write_ascii(label);
    write_ascii("_type: ");
    write_dec(desc.memory_type as u64);
    write_ascii("\r\n");
    write_ascii(label);
    write_ascii("_physical_start: 0x");
    write_hex64(desc.physical_start);
    write_ascii("\r\n");
    write_ascii(label);
    write_ascii("_pages: ");
    write_dec(desc.number_of_pages);
    write_ascii("\r\n");
    write_ascii(label);
    write_ascii("_physical_end: 0x");
    write_hex64(desc.physical_start + desc.number_of_pages * EFI_PAGE_SIZE as u64);
    write_ascii("\r\n");
}

fn memory_descriptor_contains(desc: &EfiMemoryDescriptor, address: u64) -> bool {
    let start = desc.physical_start;
    let end = start + desc.number_of_pages * EFI_PAGE_SIZE as u64;
    start <= address && address < end
}

fn print_target_memory_map_probe(bs: *mut EfiBootServices, target: u64, size: u64) -> EfiStatus {
    let mut map_size = MEMORY_MAP_MAX;
    let mut map_key = 0usize;
    let mut descriptor_size = 0usize;
    let mut descriptor_version = 0u32;
    let memory_map = core::ptr::addr_of_mut!(MEMORY_MAP) as *mut EfiMemoryDescriptor;
    let status = unsafe {
        ((*bs).get_memory_map)(
            &mut map_size,
            memory_map,
            &mut map_key,
            &mut descriptor_size,
            &mut descriptor_version,
        )
    };
    write_status("target_memory_map_status: ", status);
    if status.is_error() || descriptor_size == 0 || size == 0 {
        return status;
    }

    let target_end = target + size;
    let target_last = target_end - 1;
    write_ascii("target_memory_range_start: 0x");
    write_hex64(target);
    write_ascii("\r\n");
    write_ascii("target_memory_range_end: 0x");
    write_hex64(target_end);
    write_ascii("\r\n");

    let mut start_desc: *const EfiMemoryDescriptor = null_mut();
    let mut last_desc: *const EfiMemoryDescriptor = null_mut();
    let mut best_start = 0u64;
    let mut best_pages = 0u64;
    let count = map_size / descriptor_size;
    let mut i = 0usize;
    while i < count {
        let desc_ptr = unsafe { (memory_map as *const u8).add(i * descriptor_size) }
            as *const EfiMemoryDescriptor;
        let desc = unsafe { &*desc_ptr };
        if start_desc.is_null() && memory_descriptor_contains(desc, target) {
            start_desc = desc_ptr;
        }
        if last_desc.is_null() && memory_descriptor_contains(desc, target_last) {
            last_desc = desc_ptr;
        }
        if desc.memory_type == EFI_CONVENTIONAL_MEMORY && desc.number_of_pages > best_pages {
            best_start = desc.physical_start;
            best_pages = desc.number_of_pages;
        }
        i += 1;
    }

    print_memory_descriptor("target_start_desc", start_desc);
    print_memory_descriptor("target_last_desc", last_desc);
    write_ascii("memory_map_best_conventional_start: 0x");
    write_hex64(best_start);
    write_ascii("\r\n");
    write_ascii("memory_map_best_conventional_pages: ");
    write_dec(best_pages);
    write_ascii("\r\n");
    write_ascii("memory_map_best_conventional_end: 0x");
    write_hex64(best_start + best_pages * EFI_PAGE_SIZE as u64);
    write_ascii("\r\n");
    status
}

fn prepare_kernel_boot_info(bs: *mut EfiBootServices) -> *const OstoolKernelBootInfo {
    unsafe {
        KERNEL_BOOT_INFO = OstoolKernelBootInfo {
            magic: OSTOOL_KERNEL_BOOT_INFO_MAGIC,
            version: 1,
            framebuffer_base: 0,
            framebuffer_size: 0,
            horizontal_resolution: 0,
            vertical_resolution: 0,
            pixels_per_scan_line: 0,
            pixel_format: 0xffff_ffff,
        };
    }

    let mut gop_ptr: *mut c_void = null_mut();
    let status = unsafe {
        ((*bs).locate_protocol)(&EFI_GRAPHICS_OUTPUT_PROTOCOL_GUID, null_mut(), &mut gop_ptr)
    };
    write_status("kernel_boot_info_gop_status: ", status);
    if !status.is_error() && !gop_ptr.is_null() {
        let gop = gop_ptr as *mut EfiGraphicsOutputProtocol;
        let mode = unsafe { (*gop).mode };
        if !mode.is_null() {
            let info = unsafe { (*mode).info };
            unsafe {
                KERNEL_BOOT_INFO.framebuffer_base = (*mode).frame_buffer_base;
                KERNEL_BOOT_INFO.framebuffer_size = (*mode).frame_buffer_size as u64;
                if !info.is_null() {
                    KERNEL_BOOT_INFO.horizontal_resolution = (*info).horizontal_resolution;
                    KERNEL_BOOT_INFO.vertical_resolution = (*info).vertical_resolution;
                    KERNEL_BOOT_INFO.pixels_per_scan_line = (*info).pixels_per_scan_line;
                    KERNEL_BOOT_INFO.pixel_format = (*info).pixel_format;
                }
            }
        }
    }
    unsafe {
        write_ascii("kernel_boot_info_ptr: 0x");
        write_hex64(core::ptr::addr_of!(KERNEL_BOOT_INFO) as u64);
        write_ascii("\r\n");
        write_ascii("kernel_boot_info_fb_base: 0x");
        write_hex64(KERNEL_BOOT_INFO.framebuffer_base);
        write_ascii("\r\n");
        write_ascii("kernel_boot_info_fb_size: ");
        write_dec(KERNEL_BOOT_INFO.framebuffer_size);
        write_ascii("\r\n");
        write_ascii("kernel_boot_info_resolution: ");
        write_dec(KERNEL_BOOT_INFO.horizontal_resolution as u64);
        write_ascii("x");
        write_dec(KERNEL_BOOT_INFO.vertical_resolution as u64);
        write_ascii("\r\n");
        core::ptr::addr_of!(KERNEL_BOOT_INFO)
    }
}

fn call_kernel(entry_point: u64, boot_info: *const OstoolKernelBootInfo) -> ! {
    let entry: extern "C" fn(*const OstoolKernelBootInfo) =
        unsafe { core::mem::transmute(entry_point as usize) };
    entry(boot_info);
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
    if VERBOSE_SETUP_LOGS {
        write_prefixed_status(label, "_http_create_child_status", status);
    }
    if status.is_error() || child.is_null() {
        return if status.is_error() {
            status
        } else {
            EFI_UNSUPPORTED
        };
    }

    let http = match open_protocol::<EfiHttpProtocol>(bs, child, &EFI_HTTP_PROTOCOL_GUID) {
        Ok(http) => {
            if VERBOSE_SETUP_LOGS {
                write_prefixed_status(label, "_http_child_protocol_status", EFI_SUCCESS);
            }
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

    let boot_info = prepare_kernel_boot_info(bs);
    let status = exit_boot_services_with_fresh_map(image, bs);
    if !status.is_error() {
        call_kernel(manifest.entry_point, boot_info);
    }
    write_status("exit_boot_services_status: ", status);
    write_ascii("jump_failed\r\n");
    unsafe {
        ((*binding).destroy_child)(binding, child);
    }
    EFI_SUCCESS
}
