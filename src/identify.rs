use std::ffi::CStr;

pub fn identify(addr: usize) -> Option<&'static CStr> {
    unsafe {
        let mut info: libc::Dl_info = std::mem::zeroed();

        libc::dladdr(addr as _, &mut info);

        if !info.dli_sname.is_null() {
            let sym_name = CStr::from_ptr(info.dli_sname);
            return Some(sym_name);
        }

        /*
        let parse = |str| usize::from_str_radix(str, 16).ok();
        let maps = std::fs::read_to_string("/proc/self/maps").unwrap();
        maps.lines()
            .filter_map(|line| {
                let mut ws = line.split_ascii_whitespace();

                let addr_range = ws.next()?;
                let mut addr_range = addr_range.split("-");
                let addr_range = (parse(addr_range.next()?)?)..(parse(addr_range.next()?)?);

                if addr_range.contains(&addr) {
                    ws.nth(4).map(ToOwned::to_owned)
                } else {
                    None
                }
            })
            .next()
             */

        None
    }
}
