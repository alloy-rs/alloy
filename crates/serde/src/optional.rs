//! Serde functions for encoding optional values.

use serde::{Deserialize, Deserializer};

/// For use with serde's `deserialize_with` on a sequence that must be
/// deserialized as a single but optional (i.e. possibly `null`) value.
pub fn null_as_default<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: Deserialize<'de> + Default,
    D: Deserializer<'de>,
{
    Option::<T>::deserialize(deserializer).map(Option::unwrap_or_default)
}

/// For use with serde's `deserialize_with` on a field that must be missing.
pub fn reject_if_some<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    let value = Option::<T>::deserialize(deserializer)?;

    if value.is_some() {
        return Err(serde::de::Error::custom("unexpected value"));
    }

    Ok(value)
}

/// Deserializes a vector of optional `f64`, replacing `None` values with `0.0`.
/// Returns an empty vector if the input is `null`.
pub fn null_as_default_array<'de,D>(deserializer : D) -> Result<Vec<f64>, D::Error>
where 
    D : Deserializer<'de>
{
    let opt : Option<Vec<Option<f64>>> = Option::deserialize(deserializer)?;
    match opt{
        Some(vec) => {
            // replace None with 0.0
            Ok(vec.into_iter().map(|x| x.unwrap_or(0.0)).collect())
        }
        None => Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[derive(Debug, Deserialize, PartialEq)]
    struct TestStruct {
        #[serde(default, deserialize_with = "null_as_default")]
        value: Vec<i32>,

        #[serde(default, deserialize_with = "reject_if_some")]
        should_be_none: Option<String>,
    }

    #[test]
    fn test_null_as_default_with_null() {
        let json_data = json!({ "value": null });
        let result: TestStruct = serde_json::from_value(json_data).unwrap();
        assert_eq!(result.value, Vec::<i32>::new());
    }

    #[test]
    fn test_null_as_default_with_value() {
        let json_data = json!({ "value": [1, 2, 3] });
        let result: TestStruct = serde_json::from_value(json_data).unwrap();
        assert_eq!(result.value, vec![1, 2, 3]);
    }

    #[test]
    fn test_null_as_default_with_missing_field() {
        let json_data = json!({});
        let result: TestStruct = serde_json::from_value(json_data).unwrap();
        assert_eq!(result.value, Vec::<i32>::new());
    }

    #[test]
    fn test_reject_if_some_with_none() {
        let json_data = json!({});
        let result: TestStruct = serde_json::from_value(json_data).unwrap();
        assert_eq!(result.should_be_none, None);
    }

    #[test]
    fn test_reject_if_some_with_some() {
        let json_data = json!({ "should_be_none": "unexpected value" });
        let result: Result<TestStruct, _> = serde_json::from_value(json_data);
        assert!(result.is_err());
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct Test {
        #[serde(deserialize_with = "null_as_default_array")]
        blob_gas_used_ratio: Vec<f64>,
    }

    #[test]
    fn test_blob_gas_used_ratio_null_field() {
        let json_data = json!({"blob_gas_used_ratio":null});
        let result: Test = serde_json::from_value(json_data).unwrap();
        assert_eq!(result.blob_gas_used_ratio, Vec::<f64>::new());
    }

    #[test]
    fn test_blob_gas_used_ratio_null_elements() {
        let json_data = json!({ "blob_gas_used_ratio": [0.5, null, 0.8] });
        let result: Test = serde_json::from_value(json_data).unwrap();
        assert_eq!(result.blob_gas_used_ratio, vec![0.5, 0.0, 0.8]);
    }

    #[test]
    fn test_blob_gas_used_ratio_normal_array() {
        let json_data = json!({ "blob_gas_used_ratio": [0.1, 0.2, 0.3] });
        let result: Test = serde_json::from_value(json_data).unwrap();
        assert_eq!(result.blob_gas_used_ratio, vec![0.1, 0.2, 0.3]);
    }
}
