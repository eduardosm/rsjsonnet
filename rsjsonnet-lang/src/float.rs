pub(crate) fn try_to_u8_exact(x: f64) -> Option<u8> {
    let x_int = x as u8;
    if x_int as f64 == x {
        Some(x_int)
    } else {
        None
    }
}

pub(crate) fn try_to_u32(x: f64) -> Option<u32> {
    let x = x.trunc();
    let x_int = x as u32;
    if x_int as f64 == x {
        Some(x_int)
    } else {
        None
    }
}

pub(crate) fn try_to_i32_exact(x: f64) -> Option<i32> {
    let x_int = x as i32;
    if x_int as f64 == x {
        Some(x_int)
    } else {
        None
    }
}

pub(crate) fn try_to_usize(x: f64) -> Option<usize> {
    let x = x.trunc();
    let x_int = x as usize;
    if x_int as f64 == x {
        Some(x_int)
    } else {
        None
    }
}

pub(crate) fn try_to_usize_exact(x: f64) -> Option<usize> {
    let x_int = x as usize;
    if x_int as f64 == x {
        Some(x_int)
    } else {
        None
    }
}
