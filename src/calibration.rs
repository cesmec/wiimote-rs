#[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)] // Numbers will not be that large
pub fn normalize<TValue, TResult>(
    value: TValue,
    value_bits: usize,
    zero: TValue,
    max: TValue,
    calibration_bits: usize,
) -> TResult
where
    TValue: std::ops::Shl<usize, Output = TValue> + Into<TResult> + Copy,
    TResult: std::ops::Sub<Output = TResult> + std::ops::Div<Output = TResult> + Copy,
{
    let missing_calibration_bits =
        isize::max(0, value_bits as isize - calibration_bits as isize) as usize;
    let missing_value_bits =
        isize::max(0, calibration_bits as isize - value_bits as isize) as usize;

    let value = value << missing_value_bits;
    let zero = zero << missing_calibration_bits;
    let max = max << missing_calibration_bits;

    (Into::<TResult>::into(value) - Into::<TResult>::into(zero))
        / (Into::<TResult>::into(max) - Into::<TResult>::into(zero))
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    (1.0 - t).mul_add(a, b * t)
}

pub fn inverse_lerp(a: f32, b: f32, t: f32) -> f32 {
    (t - a) / (b - a)
}

pub fn remap(value: f32, from_start: f32, from_end: f32, to_start: f32, to_end: f32) -> f32 {
    let t = inverse_lerp(from_start, from_end, value);
    lerp(to_start, to_end, t)
}
