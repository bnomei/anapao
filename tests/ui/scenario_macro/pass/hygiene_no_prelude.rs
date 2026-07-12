#![no_implicit_prelude]

extern crate anapao;
extern crate std;

fn main() {
    let __anapao_builder = 1_u8;
    let __anapao_value = 2_u8;
    let __anapao_node_ids = 3_u8;
    let result = ::anapao::scenario! {
        id: "hygiene";
        nodes { source: Source { initial: 1.0 }; sink: Sink; }
        edges { flow: source -> sink => remaining; }
    };
    ::std::assert!(::std::result::Result::is_ok(&result));
    ::std::assert_eq!((__anapao_builder, __anapao_value, __anapao_node_ids), (1, 2, 3));
}
