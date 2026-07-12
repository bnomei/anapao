fn main() {
    let _ = anapao::scenario! { id: "bad"; nodes { node: Pool { config: anapao::types::PoolConfig::default(), capacity: 1 }; } edges {} };
}
