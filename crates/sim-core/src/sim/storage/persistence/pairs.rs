//! JSON object keys cannot be tuples. Preserve map order using entry pairs.
use serde::{Deserialize, Deserializer, Serialize, Serializer};
pub fn serialize<'a, M, K: Serialize + 'a, V: Serialize + 'a, S: Serializer>(
    map: &'a M,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    &'a M: IntoIterator<Item = (&'a K, &'a V)>,
{
    map.into_iter().collect::<Vec<_>>().serialize(serializer)
}
pub fn deserialize<'de, M, K: Deserialize<'de>, V: Deserialize<'de>, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<M, D::Error>
where
    M: FromIterator<(K, V)>,
{
    Ok(Vec::<(K, V)>::deserialize(deserializer)?.into_iter().collect())
}
