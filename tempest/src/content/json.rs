pub fn parse(input: &[u8]) -> anyhow::Result<serde_json::Value> {
    Ok(serde_json::from_slice::<serde_json::Value>(input)?)
}
