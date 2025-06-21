use windows_registry::{Key, Type, Value};

pub fn get_default_keys() -> [(&'static Key, &'static str); 5] {
    [
        (windows_registry::CLASSES_ROOT, "HKEY_CLASSES_ROOT"),
        (windows_registry::CURRENT_USER, "HKEY_CURRENT_USER"),
        (windows_registry::LOCAL_MACHINE, "HKEY_LOCAL_MACHINE"),
        (windows_registry::USERS, "HKEY_USERS"),
        (windows_registry::CURRENT_CONFIG, "HKEY_CURRENT_CONFIG")
    ]
}

pub fn read_key(key: &Key, path: &str) -> windows_registry::Result<Key> {
    let key = key
        .options()
        .read()
        .open(path);

    key
}

pub fn read_subkeys(key: &Key) -> windows_registry::Result<Vec<String>> {
    key.keys().map(|keys| keys.collect())
}

pub fn read_values(key: &Key) -> windows_registry::Result<Vec<(String, Value)>> {
    key.values().map(|values| values.collect())
}

pub fn get_printable_type(t: Type) -> &'static str {
    match t {
        Type::Bytes => "REG_BINARY",
        Type::String => "REG_SZ",
        Type::ExpandString => "REG_EXPAND_SZ",
        Type::MultiString => "REG_MULTI_SZ",
        Type::U32 => "REG_DWORD",
        Type::U64 => "REG_QWORD",
        Type::Other(_) => "REG_NONE",
    }
}

pub fn get_printable_value(value: &Value) -> String {
    match value.ty() {
        Type::Bytes => "TODO".into(),
        _ => "TODO".into(),
    }
}
