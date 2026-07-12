fn main() {
    anapao::scenario! {
        id: "queue-flow";

        nodes {
            source: Source { initial: 64.0 };
            delay: Delay { steps: 2 };
            sink: Pool;
        }
        edges {
            source_delay: source -> delay => fixed(1.0);
            delay_sink: delay -> sink => remaining;
        }
    }
    .unwrap();
}
