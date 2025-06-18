use windows_registry::Key;

pub fn get_default_keys() -> [(&'static Key, &'static str); 5] {
    [
        (windows_registry::CLASSES_ROOT, "HKEY_CLASSES_ROOT"),
        (windows_registry::CURRENT_USER, "HKEY_CURRENT_USER"),
        (windows_registry::LOCAL_MACHINE, "HKEY_LOCAL_MACHINE"),
        (windows_registry::USERS, "HKEY_USERS"),
        (windows_registry::CURRENT_CONFIG, "HKEY_CURRENT_CONFIG")
    ]
}

pub fn read_key(key: &Key, path: &str) -> Key {
    let key = key
        .options()
        .read()
        .open(path)
        .unwrap();

    key
}

pub fn read_subkeys(key: &Key) -> Vec<String> {
    key.keys().unwrap().collect()
}
