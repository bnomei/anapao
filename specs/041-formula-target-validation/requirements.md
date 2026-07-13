# Requirements

R001: WHEN a state connection has `StateConnectionTarget::Formula` with an absent
`target_connection` edge ID, THE SYSTEM SHALL reject compilation with the same
`SetupError::InvalidGraphReference` shape and available-edge hint used by resource and state
connection targets.

R002: WHEN `scenario!` receives `target: formula(absent)`, THE SYSTEM SHALL surface the
checked-builder missing-edge error without macro-side registry checks or altered macro grammar.
