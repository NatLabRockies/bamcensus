/// Formats a Census identifier as its canonical zero-padded FIPS/GEOID string
/// (for example, `"08031"` for Denver County, Colorado), without the `<type>=`
/// prefix included in [`std::fmt::Display`] output.
pub trait HasGeoidString {
    fn geoid_string(&self) -> String;
}
