use anapao::prelude::*;

fn main() {
    let _: Result<Scenario, anapao::error::SetupError> = anapao::scenario! {
        id: "prelude";
        nodes {}
        edges {}
    };
}
