/// A count as the screens and the export write it: an absurd one loses its exact figure rather
/// than failing a command over a label.
pub(crate) fn saturating_u32<T: TryInto<u32>>(value: T) -> u32 {
    value.try_into().unwrap_or(u32::MAX)
}
