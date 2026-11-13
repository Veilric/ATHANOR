//! Lossless, OS-specific path encoding for local caches (not a portable interchange format).
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::path::{Path, PathBuf};

pub fn serialize<S: Serializer>(path: &Path, serializer: S) -> Result<S::Ok, S::Error> {
    if let Some(text) = path.to_str() {
        return serializer.serialize_str(text);
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        path.as_os_str().as_bytes().serialize(serializer)
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        path.as_os_str()
            .encode_wide()
            .collect::<Vec<_>>()
            .serialize(serializer)
    }
}
pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<PathBuf, D::Error> {
    #[cfg(unix)]
    type Units = u8;
    #[cfg(windows)]
    type Units = u16;
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Encoded {
        Text(String),
        Native(Vec<Units>),
    }
    let units = match Encoded::deserialize(deserializer)? {
        Encoded::Text(text) => return Ok(text.into()),
        Encoded::Native(units) => units,
    };
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        Ok(std::ffi::OsString::from_vec(units).into())
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStringExt;
        Ok(std::ffi::OsString::from_wide(&units).into())
    }
}
