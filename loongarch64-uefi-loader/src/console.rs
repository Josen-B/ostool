static mut CONSOLE: *mut EfiSimpleTextOutputProtocol = null_mut();
static mut SERIAL: *mut EfiSerialIoProtocol = null_mut();

impl EfiStatus {
    fn is_error(self) -> bool {
        (self.0 & EFI_ERROR_BIT) != 0
    }
}

fn console() -> *mut EfiSimpleTextOutputProtocol {
    unsafe { CONSOLE }
}

fn configure_serial_output(bs: *mut EfiBootServices) {
    let mut serial_ptr: *mut c_void = null_mut();
    let status = unsafe {
        ((*bs).locate_protocol)(&EFI_SERIAL_IO_PROTOCOL_GUID, null_mut(), &mut serial_ptr)
    };
    if !status.is_error() && !serial_ptr.is_null() {
        unsafe {
            SERIAL = serial_ptr as *mut EfiSerialIoProtocol;
        }
    }
    write_status("serial_io_locate_status: ", status);
    write_ascii("serial_io_enabled: ");
    write_ascii(if unsafe { SERIAL }.is_null() {
        "no\r\n"
    } else {
        "yes\r\n"
    });
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
    let serial = unsafe { SERIAL };
    if !serial.is_null() {
        let mut written = s.len();
        unsafe {
            ((*serial).write)(serial, &mut written, s.as_ptr() as *mut c_void);
        }
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
