fn main() {
    let _ = anapao::scenario! { id: "bad"; nodes { node: Source { mystery: 1 }; } edges {} };
}
