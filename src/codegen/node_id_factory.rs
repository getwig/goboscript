use rustc_hash::FxHashSet;

use super::node_id::NodeID;

/// Allocates block ids. By default ids are sequential per target. With `stable` set, ids are
/// derived from the target name, a key for the enclosing script (`begin_script`) and the
/// allocation order within that script, so editing one script does not change the ids of
/// blocks in other scripts or targets (GBS-022).
#[derive(Debug)]
pub struct NodeIDFactory {
    value: u64,
    pub stable: bool,
    scope: u64,
    used: FxHashSet<u64>,
}

impl Default for NodeIDFactory {
    fn default() -> Self {
        Self::new()
    }
}

/// 64-bit FNV-1a; stable across platforms and Rust versions (unlike `DefaultHasher`).
fn fnv1a(seed: u64, bytes: &[u8]) -> u64 {
    let mut hash = seed;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
/// Stable ids are kept to 60 bits (at most 10 characters after the `$`).
const STABLE_MASK: u64 = (1 << 60) - 1;

impl NodeIDFactory {
    pub fn new() -> Self {
        Self {
            value: 0,
            stable: false,
            scope: FNV_OFFSET,
            used: Default::default(),
        }
    }

    /// Start a new target.
    pub fn reset(&mut self) {
        self.value = 0;
        self.scope = FNV_OFFSET;
        self.used.clear();
    }

    /// Start a top-level script of `target` identified by `key` (stable ids only).
    pub fn begin_script(&mut self, target: &str, key: &str) {
        if !self.stable {
            return;
        }
        self.scope = fnv1a(fnv1a(FNV_OFFSET, target.as_bytes()), b"\0");
        self.scope = fnv1a(self.scope, key.as_bytes());
        self.value = 0;
    }

    pub fn new_id(&mut self) -> NodeID {
        let value = self.value;
        self.value += 1;
        if !self.stable {
            return NodeID::new(value);
        }
        let mut id = fnv1a(self.scope, &value.to_le_bytes()) & STABLE_MASK;
        while !self.used.insert(id) {
            id = fnv1a(id, b"\x01") & STABLE_MASK;
        }
        NodeID::new(id)
    }
}

impl Iterator for NodeIDFactory {
    type Item = NodeID;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.new_id())
    }
}
