// TODO: Add auto-populated blocklist.
// Includes `pub static BLOCKED: [&str; N] = [...];`
// include!(concat!(env!("BLOCKLIST")));

pub const BLOCKED: &[&str] = &[
    // MUST BE SORTED FOR BINARY SEARCH TO WORK.
];

pub const LOCAL_MAPPINGS: &[(&str, [u8; 4])] = &[
    ("server.lan", [192, 168, 86, 89]),
];

pub fn is_blocked(name: &str) -> bool {
    // Exact match via O(log N) Binary Search
    if BLOCKED.binary_search(&name).is_ok() {
        return true;
    }

    // Subdomain check: iteratively strip leading labels and check binary search
    let mut parts = name.split_once('.');
    while let Some((_, rest)) = parts {
        if BLOCKED.binary_search(&rest).is_ok() {
            return true;
        }
        parts = rest.split_once('.');
    }

    false
}

pub fn local_mapping(name: &str) -> Option<[u8; 4]> {
    LOCAL_MAPPINGS.iter().find(|(d, _)| *d == name).map(|(_, addr)| *addr)
}
