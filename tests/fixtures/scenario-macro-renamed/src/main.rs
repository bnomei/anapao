fn main() {
    simulation::scenario! {
        id: "renamed-dependency";
        nodes { source: Source; sink: Sink; }
        edges { flow: source -> sink => remaining; }
    }
    .unwrap();
}
