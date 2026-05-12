pub mod header_map_serde {
    use std::collections::HashMap;
    use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::str::FromStr;

    pub fn serialize<S: Serializer>(headers: &HeaderMap, s: S) -> Result<S::Ok, S::Error> {
        let map: HashMap<String, Vec<String>> = {
            let mut m = HashMap::new();
            for (k, v) in headers.iter() {
                m.entry(k.to_string())
                    .or_insert_with(Vec::new)
                    .push(v.to_str().unwrap_or("").to_string());
            }
            m
        };
        map.serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<HeaderMap, D::Error> {
        let map = HashMap::<String, Vec<String>>::deserialize(d)?;
        let mut headers = HeaderMap::new();
        for (k, values) in map {
            let name = HeaderName::from_str(&k).map_err(serde::de::Error::custom)?;
            for v in values {
                let value = HeaderValue::from_str(&v).map_err(serde::de::Error::custom)?;
                headers.append(name.clone(), value);
            }
        }
        Ok(headers)
    }
}