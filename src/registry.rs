use windows_registry::{Key, Type, Value};

pub const DEFAULT_KEYS: [(&'static Key, &'static str); 5] = [
    (windows_registry::CLASSES_ROOT, "HKEY_CLASSES_ROOT"),
    (windows_registry::CURRENT_USER, "HKEY_CURRENT_USER"),
    (windows_registry::LOCAL_MACHINE, "HKEY_LOCAL_MACHINE"),
    (windows_registry::USERS, "HKEY_USERS"),
    (windows_registry::CURRENT_CONFIG, "HKEY_CURRENT_CONFIG")
];

pub const TYPE_STRINGS: [&str; 7] = [
    "REG_BINARY",
    "REG_SZ",
    "REG_EXPAND_SZ",
    "REG_MULTI_SZ",
    "REG_DWORD",
    "REG_QWORD",
    "REG_NONE",
];

pub fn get_type_strings_vec() -> Vec<String> {
    TYPE_STRINGS.iter().map(|&s| s.to_string()).collect()
}

pub fn get_type_choices_vec() -> Vec<String> {
    TYPE_STRINGS[..TYPE_STRINGS.len()-1]
        .iter()
        .map(|&s| s.to_string())
        .collect()
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

pub fn clone_key(key: &Key) -> Key {
    key.open("").expect("Same key wasn't able to be cloned")
}

pub fn new_key(key: &Key, name: impl AsRef<str>) -> windows_registry::Result<()> {
    key.create(name).map(|_| ())
}

pub fn rename_key(key: &Key, original: impl AsRef<str>, new: impl AsRef<str>) -> windows_registry::Result<()> {
    key.rename(original, new).map(|_| ())
}

pub fn delete_key(key: &Key, name: impl AsRef<str>) -> windows_registry::Result<()> {
    key.remove_tree(name).map(|_| ())
}

pub fn read_values(key: &Key) -> windows_registry::Result<Vec<(String, Value)>> {
    key.values().map(|values| values.collect())
}

pub fn type_to_str(t: Type) -> &'static str {
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

pub fn str_to_type(s: &str) -> Type {
    match s {
        "REG_BINARY" => Type::Bytes,
        "REG_SZ" => Type::String,
        "REG_EXPAND_SZ" => Type::ExpandString,
        "REG_MULTI_SZ" => Type::MultiString,
        "REG_DWORD" => Type::U32,
        "REG_QWORD" => Type::U64,
        "REG_NONE" => Type::Other(0),
        _ => Type::Other(0),
    }
}

fn get_printable_binary(value: &Value) -> String {
    value.iter().map(|byte| format!("{:02x}", byte)).collect::<Vec<String>>().join(" ")
}

fn get_printable_sz(value: &Value) -> String {
    let wide = value.as_wide();
    let wstr = match wide.iter().position(|&c| c == 0) {
        Some(pos) => &wide[..pos],
        None => wide,
    };

    String::from_utf16_lossy(wstr).to_string()
}

fn get_printable_multi_sz(value: &Value) -> String {
    let mut strs = Vec::new();
    let mut current = Vec::new();

    for &u in value.as_wide() {
        match u {
            0 => {
                if current.is_empty() {
                    break;
                }

                strs.push(String::from_utf16_lossy(&current));
                current.clear();
            }
            _ => current.push(u),
        }
    }

    strs.join(" ").trim_end().to_string()
}

fn get_printable_u32(value: &Value) -> String {
    let num = u32::from_le_bytes(*&value[..4].try_into().unwrap());

    format!("{:#010x} ({})", num, num)
}

fn get_printable_u64(value: &Value) -> String {
    let num = u64::from_le_bytes(*&value[..8].try_into().unwrap());

    format!("{:#010x} ({})", num, num)
}

pub fn get_printable_value(value: &Value) -> String {
    match value.ty() {
        Type::Bytes => get_printable_binary(value),
        Type::String | Type::ExpandString => get_printable_sz(value),
        Type::MultiString => get_printable_multi_sz(value),
        Type::U32 if value.len() >= 4 => get_printable_u32(value),
        Type::U64 if value.len() >= 8 => get_printable_u64(value),
        _ => "(unknown data)".into(),
    }
}

pub fn validate_value_data(ty: windows_registry::Type, value: &str) -> Result<(), ()> {
    Ok(())
}

pub fn set_value(key: &Key, name: impl AsRef<str>, ty: windows_registry::Type, value: &str) -> Result<(), ()> {
    match ty {
        Type::Bytes => (),
        Type::String | Type::ExpandString => (),
        Type::MultiString => (),
        Type::U32 => (),
        Type::U64 => (),
        _ => (),
    };

    Ok(())
}
