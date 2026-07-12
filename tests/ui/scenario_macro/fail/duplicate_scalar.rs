fn main() {
    let _ = anapao::scenario! { id: "bad"; nodes { node: Source { initial: 1.0, initial: 2.0 }; } edges {} };
}
