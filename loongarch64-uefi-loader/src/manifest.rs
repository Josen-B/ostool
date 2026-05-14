fn find_key<'a>(json: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let mut p = 0;
    while p < json.len() {
        if json[p] != b'"' {
            p += 1;
            continue;
        }
        let mut i = 0;
        while i < key.len() && p + 1 + i < json.len() && json[p + 1 + i] == key[i] {
            i += 1;
        }
        if i == key.len() && p + 1 + i < json.len() && json[p + 1 + i] == b'"' {
            let mut q = p + 2 + key.len();
            while q < json.len() && matches!(json[q], b' ' | b'\r' | b'\n' | b'\t') {
                q += 1;
            }
            if q < json.len() && json[q] == b':' {
                return Some(&json[q + 1..]);
            }
        }
        p += 1;
    }
    None
}

fn json_string(json: &[u8], key: &[u8], out: &mut [u8]) -> Result<usize, ()> {
    let mut p = find_key(json, key).ok_or(())?;
    while !p.is_empty() && matches!(p[0], b' ' | b'\r' | b'\n' | b'\t') {
        p = &p[1..];
    }
    if p.is_empty() || p[0] != b'"' {
        return Err(());
    }
    p = &p[1..];
    let mut n = 0;
    while !p.is_empty() && p[0] != b'"' {
        if p[0] == b'\\' || n + 1 >= out.len() {
            return Err(());
        }
        out[n] = p[0];
        n += 1;
        p = &p[1..];
    }
    if p.is_empty() || p[0] != b'"' {
        return Err(());
    }
    if n < out.len() {
        out[n] = 0;
    }
    Ok(n)
}

fn parse_u64(mut s: &[u8]) -> Result<u64, ()> {
    let mut value = 0u64;
    let mut radix = 10u64;
    let mut saw = false;
    if s.len() >= 2 && s[0] == b'0' && (s[1] == b'x' || s[1] == b'X') {
        radix = 16;
        s = &s[2..];
    }
    let mut i = 0;
    while i < s.len() {
        if s[i] == b'_' {
            i += 1;
            continue;
        }
        let digit = match s[i] {
            b'0'..=b'9' => s[i] - b'0',
            b'a'..=b'f' => s[i] - b'a' + 10,
            b'A'..=b'F' => s[i] - b'A' + 10,
            _ => break,
        };
        if digit as u64 >= radix {
            return Err(());
        }
        value = value * radix + digit as u64;
        saw = true;
        i += 1;
    }
    if saw {
        Ok(value)
    } else {
        Err(())
    }
}

fn json_u64(json: &[u8], key: &[u8]) -> Result<u64, ()> {
    let mut p = find_key(json, key).ok_or(())?;
    while !p.is_empty() && matches!(p[0], b' ' | b'\r' | b'\n' | b'\t') {
        p = &p[1..];
    }
    parse_u64(p)
}

fn json_addr_string(json: &[u8], key: &[u8]) -> Result<u64, ()> {
    let mut buf = [0u8; 64];
    let len = json_string(json, key, &mut buf)?;
    parse_u64(&buf[..len])
}

fn parse_manifest(json: &[u8]) -> Result<Manifest, ()> {
    let mut manifest = Manifest {
        kernel_url: [0; 1024],
        kernel_url_len: 0,
        kernel_size: 0,
        kernel_load_addr: 0,
        entry_point: 0,
        arch: [0; 32],
        arch_len: 0,
    };
    manifest.kernel_url_len = json_string(json, b"kernel_url", &mut manifest.kernel_url)?;
    manifest.kernel_size = json_u64(json, b"kernel_size")?;
    manifest.kernel_load_addr = json_addr_string(json, b"kernel_load_addr")?;
    manifest.entry_point = json_addr_string(json, b"entry_point")?;
    manifest.arch_len = json_string(json, b"arch", &mut manifest.arch)?;
    Ok(manifest)
}
