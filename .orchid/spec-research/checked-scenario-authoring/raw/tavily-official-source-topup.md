# Tavily official-source top-up

Run date: 2026-07-11
Search depth: advanced

## Nonzero and serde

Query: `Rust NonZeroU64 serde Serialize Deserialize integer representation`
Request ID: `62c5dc6f-f62c-4bc0-a07b-d99aaa779f97`

Results established that `NonZeroU64` represents a `u64` whose valid values exclude zero and that
serde implements both `Serialize` and `Deserialize` for it. The design nevertheless keeps plain
`u64` in legacy DTO fields and converts to `NonZeroU64` after deserialization, avoiding any change
to legacy error timing or DTO types.

- <https://doc.rust-lang.org/std/num/type.NonZeroU64.html>
- <https://docs.rs/serde/latest/serde/trait.Serialize.html>
- <https://docs.rs/serde/latest/serde/trait.Deserialize.html>

## Diagnostic `must_use`

Query: `Rust must_use attribute functions methods types custom message`
Request ID: `f8b64d7f-8e85-4040-b4b3-12ded2b392c3`

The Rust Reference result confirms that `#[must_use]` can annotate types and functions/methods and
can carry a custom diagnostic message. This supports annotating the checked builder type and every
consuming scenario-authoring `with_*` method.

- <https://doc.rust-lang.org/reference/attributes/diagnostics.html>
- <https://rust-lang.github.io/rfcs/1940-must-use-functions.html>

## Semver and public sums

Query: `public struct field private breaking enum variant`
Request ID: `08c2cb84-d0dd-4445-a45b-8f72579c083b`

Cargo's SemVer reference classifies adding a variant to an exhaustive public enum as a major
change and documents the field-visibility cases that make struct evolution breaking. New checked
enums therefore start `#[non_exhaustive]`; existing serde DTO declarations remain unchanged.

- <https://doc.rust-lang.org/cargo/reference/semver.html>

## Conventional builder precedent

Query: `pub fn arg &mut self Command builder`
Request ID: `35931694-150f-4678-bbe7-fcd29f6f7225`

The standard `Command` API exposes repeated configuration via `&mut self -> &mut Command`. The
checked scenario design supports mutation-style `insert_*` methods and additionally retains
consuming `with_*` conveniences because that style already exists throughout Anapao.

- <https://doc.rust-lang.org/std/process/struct.Command.html>

## Serde enum representation

Query: `enum representations externally tagged internally tagged adjacently tagged untagged`
Request ID: `d6f9faf4-26b5-4c84-9a89-67f80aff7f55`

Serde documents four selectable wire representations and notes that untagged matching is
order-dependent. Because Anapao already pins a tagged DTO representation, the checked sums do not
derive serde and do not replace that established wire contract.

- <https://serde.rs/enum-representations.html>
- <https://serde.rs/container-attrs.html>

## Fallible conversion

The PRO report returned the standard `TryFrom` direction; the canonical API definition is:

- <https://doc.rust-lang.org/std/convert/trait.TryFrom.html>

## Tool note

Two attempts to request Tavily raw-page expansion failed because the installed CLI passed
`include_raw_content = "markdown"` to a backend expecting a boolean. The searches were rerun
without raw expansion and completed successfully. This did not affect result discovery or source
URLs.
