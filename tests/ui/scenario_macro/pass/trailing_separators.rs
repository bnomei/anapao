fn main() {
    anapao::scenario! {
        id: "trailing";
        tags ["one",];
        metadata {"one" => "two",}
        nodes {
            source: Source { tags ["tag",], metadata {"k" => "v",}, };
            sink: Sink;
        }
        edges {
            flow: source -> sink => remaining resource { metadata {"k" => "v",}, };
            state: source -> sink => remaining state { target: node, metadata {"k" => "v",}, };
        }
        track [source, sink,];
        end any [max_steps(1),];
    }.unwrap();
}
