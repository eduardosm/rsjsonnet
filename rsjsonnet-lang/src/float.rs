pub(crate) fn try_to_u8_exact(x: f64) -> Option<u8> {
    let x_int = x as u8;
    if x_int as f64 == x { Some(x_int) } else { None }
}

pub(crate) fn try_to_u32(x: f64) -> Option<u32> {
    let x = x.trunc();
    let x_int = x as u32;
    if x_int as f64 == x { Some(x_int) } else { None }
}

pub(crate) fn try_to_i32_exact(x: f64) -> Option<i32> {
    let x_int = x as i32;
    if x_int as f64 == x { Some(x_int) } else { None }
}

pub(crate) fn try_to_usize(x: f64) -> Option<usize> {
    let x = x.trunc();
    let x_int = x as usize;
    if x_int as f64 == x { Some(x_int) } else { None }
}

pub(crate) fn try_to_usize_exact(x: f64) -> Option<usize> {
    let x_int = x as usize;
    if x_int as f64 == x { Some(x_int) } else { None }
}

pub(crate) fn frexp(x: f64) -> (f64, i16) {
    let (norm, edelta) = if x.is_subnormal() {
        let scale = f64::from_bits((52 + 0x3FF) << 52); // 2^52
        (x * scale, -52)
    } else {
        (x, 0)
    };
    let raw = norm.to_bits();
    let raw_exp = (raw >> 52) & 0x7FF;
    if raw_exp == 0 {
        let mant = f64::from_bits(raw & !0x7FFF_FFFF_FFFF_FFFF);
        let exp = 0;
        (mant, exp)
    } else {
        let mant = f64::from_bits((raw & !(0x7FF << 52)) | (0x3FE << 52));
        let exp = raw_exp as i16 - 0x3FE + edelta;
        (mant, exp)
    }
}

#[cfg(test)]
mod tests {
    use super::frexp;

    #[test]
    fn test_frexp() {
        assert_eq!(frexp(0.0), (0.0, 0));
        assert_eq!(frexp(0.09375), (0.75, -3));
        assert_eq!(frexp(-0.09375), (-0.75, -3));
        assert_eq!(frexp(0.25), (0.5, -1));
        assert_eq!(frexp(-0.25), (-0.5, -1));
        assert_eq!(frexp(0.5), (0.5, 0));
        assert_eq!(frexp(-0.5), (-0.5, 0));
        assert_eq!(frexp(1.0), (0.5, 1));
        assert_eq!(frexp(-1.0), (-0.5, 1));
        assert_eq!(frexp(20.0), (0.625, 5));
        assert_eq!(frexp(-20.0), (-0.625, 5));

        // Subnormal numbers
        assert_eq!(frexp(f64::from_bits(0b01)), (0.5, -1073));
        assert_eq!(frexp(-f64::from_bits(0b01)), (-0.5, -1073));
        assert_eq!(frexp(f64::from_bits(0b10)), (0.5, -1072));
        assert_eq!(frexp(-f64::from_bits(0b10)), (-0.5, -1072));
        assert_eq!(frexp(f64::from_bits(0b11)), (0.75, -1072));
        assert_eq!(frexp(-f64::from_bits(0b11)), (-0.75, -1072));
    }
}
