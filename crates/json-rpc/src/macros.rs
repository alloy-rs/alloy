/// A macro to check if a field is present in a map and set it if it is, else return an error.
macro_rules! check_and_set_field {
    ($map:expr, $field:expr) => {{
        if $field.is_some() {
            return Err(serde::de::Error::duplicate_field(stringify!($field)));
        }
        $field = Some($map.next_value()?);
    }};
}
