use std::cell::Cell;

fn once<T>(count: &Cell<usize>, value: T) -> T {
    count.set(count.get() + 1);
    value
}

fn main() {
    let count = Cell::new(0);
    let scenario = anapao::scenario! {
        id: once(&count, "one-evaluation");
        title: once(&count, "title");
        tags [once(&count, "tag")];
        metadata {once(&count, "key") => once(&count, "value")}
        nodes {
            source: Source { initial: once(&count, 2.0), label: once(&count, "source") };
            sink: Sink;
        }
        edges {
            flow: source -> sink => fixed(once(&count, 1.0)) resource { enabled: once(&count, true) };
        }
        end max_steps(once(&count, 1));
    }.unwrap();
    assert_eq!(scenario.id().as_str(), "one-evaluation");
    assert_eq!(count.get(), 10);
}
