/// Specta refuses `usize` and `i64`, which JSON cannot carry exactly. An absurd count loses
/// its exact figure rather than failing a command over a label.
pub(crate) fn saturating_u32<T: TryInto<u32>>(value: T) -> u32 {
    value.try_into().unwrap_or(u32::MAX)
}
