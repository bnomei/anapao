fn main() {
    let _ = anapao::scenario! { id: "bad"; nodes { a: Source; b: Sink; } edges { flow: a -> b => remaining state { target: resource_connection("flow") }; } };
}
