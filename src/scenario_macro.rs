//! Declarative checked scenario authoring.

/// Authors a complete checked [`Scenario`](crate::types::Scenario) using symbolic node and edge
/// names.
///
/// The macro is an ergonomic front end for [`ScenarioBuilder`](crate::types::ScenarioBuilder):
/// captured expressions are evaluated once, identifiers come from the exact spelling of their
/// symbols, and whole-graph validation remains owned by `ScenarioBuilder::build`.
#[macro_export]
macro_rules! scenario {
    // Public entry. Section order is intentionally fixed so malformed input has a useful error.
    (
        id: $id:expr;
        $(title: $title:expr;)?
        $(description: $description:expr;)?
        $(tags [$($scenario_tag:expr),* $(,)?];)?
        $(variables: $variables:expr;)?
        $(metadata {$($scenario_key:expr => $scenario_value:expr),* $(,)?})?
        nodes {$($nodes:tt)*}
        edges {$($edges:tt)*}
        $($tail:tt)*
    ) => {{
        (|| -> ::std::result::Result<$crate::types::Scenario, $crate::error::SetupError> {
            let __anapao_id_value = $id;
            let __anapao_id: $crate::types::ScenarioId =
                ::std::convert::TryInto::<$crate::types::ScenarioId>::try_into(__anapao_id_value)
                    .map_err(|error| {
                    $crate::error::SetupError::InvalidParameter {
                        name: ::std::convert::From::from("id"),
                        reason: ::std::string::ToString::to_string(&error),
                    }
                    })?;

            // Each declaration owns one retained typed identity. Metrics are node-backed but use
            // their distinct public type. Vectors deliberately preserve duplicate declarations so
            // insertion, not a macro-side map, owns duplicate errors.
            let mut __anapao_node_ids: ::std::vec::Vec<(&str, $crate::types::NodeId)> =
                ::std::vec::Vec::new();
            let mut __anapao_metric_ids: ::std::vec::Vec<(&str, $crate::types::MetricKey)> =
                ::std::vec::Vec::new();
            $crate::scenario!(@__anapao_register_nodes
                __anapao_node_ids, __anapao_metric_ids; $($nodes)*
            );
            let mut __anapao_edge_ids: ::std::vec::Vec<(&str, $crate::types::EdgeId)> =
                ::std::vec::Vec::new();
            $crate::scenario!(@__anapao_register_edges __anapao_edge_ids; $($edges)*);

            let __anapao_node = |name: &str| {
                let mut __anapao_iter = __anapao_node_ids.iter();
                if let ::std::option::Option::Some((_, id)) = ::std::iter::Iterator::find(
                    &mut __anapao_iter,
                    |(candidate, _)| *candidate == name,
                )
                {
                    ::std::result::Result::Ok(::std::clone::Clone::clone(id))
                } else {
                    $crate::types::NodeId::new(name).map_err(|error| {
                        $crate::error::SetupError::InvalidParameter {
                            name: ::std::format!("nodes.{name}.id"),
                            reason: ::std::string::ToString::to_string(&error),
                        }
                    })
                }
            };
            let __anapao_metric = |name: &str| {
                let mut __anapao_iter = __anapao_metric_ids.iter();
                if let ::std::option::Option::Some((_, id)) = ::std::iter::Iterator::find(
                    &mut __anapao_iter,
                    |(candidate, _)| *candidate == name,
                )
                {
                    ::std::result::Result::Ok(::std::clone::Clone::clone(id))
                } else {
                    $crate::types::MetricKey::new(name).map_err(|error| {
                        $crate::error::SetupError::InvalidParameter {
                            name: ::std::format!("metrics.{name}.id"),
                            reason: ::std::string::ToString::to_string(&error),
                        }
                    })
                }
            };
            let __anapao_edge = |name: &str| {
                let mut __anapao_iter = __anapao_edge_ids.iter();
                if let ::std::option::Option::Some((_, id)) = ::std::iter::Iterator::find(
                    &mut __anapao_iter,
                    |(candidate, _)| *candidate == name,
                )
                {
                    ::std::result::Result::Ok(::std::clone::Clone::clone(id))
                } else {
                    $crate::types::EdgeId::new(name).map_err(|error| {
                        $crate::error::SetupError::InvalidParameter {
                            name: ::std::format!("edges.{name}.id"),
                            reason: ::std::string::ToString::to_string(&error),
                        }
                    })
                }
            };

            let mut __anapao_builder = $crate::types::ScenarioBuilder::new(__anapao_id);
            $(
                let __anapao_value = $title;
                __anapao_builder = __anapao_builder.with_title(__anapao_value);
            )?
            $(
                let __anapao_value = $description;
                __anapao_builder = __anapao_builder.with_description(__anapao_value);
            )?
            $($(
                let __anapao_value = $scenario_tag;
                __anapao_builder = __anapao_builder.with_tag(__anapao_value);
            )*)?
            $(
                let __anapao_value = $variables;
                __anapao_builder = __anapao_builder.with_variables(__anapao_value);
            )?
            $($(
                let __anapao_key = $scenario_key;
                let __anapao_value = $scenario_value;
                __anapao_builder =
                    __anapao_builder.with_metadata(__anapao_key, __anapao_value);
            )*)?

            $crate::scenario!(@__anapao_build_nodes
                __anapao_builder, __anapao_node; $($nodes)*
            );
            $crate::scenario!(@__anapao_build_edges
                __anapao_builder, __anapao_node, __anapao_metric, __anapao_edge; $($edges)*
            );
            let mut __anapao_ends: ::std::vec::Vec<$crate::types::EndConditionSpec> =
                ::std::vec::Vec::new();
            $crate::scenario!(@__anapao_tail
                __anapao_builder, __anapao_ends, __anapao_node, __anapao_metric; $($tail)*
            );
            if !__anapao_ends.is_empty() {
                __anapao_builder = __anapao_builder.with_end_conditions(__anapao_ends);
            }
            __anapao_builder.build()
        })()
    }};

    // Identity pre-registration. The declaration parsers below perform the syntax check too.
    (@__anapao_register_nodes $nodes:ident, $metrics:ident;) => {};
    (@__anapao_register_nodes $nodes:ident, $metrics:ident;
        $name:ident : Custom($family:expr) {$($props:tt)*}; $($rest:tt)*) => {
        $crate::scenario!(@__anapao_register_one_node $nodes, $metrics, $name);
        $crate::scenario!(@__anapao_register_nodes $nodes, $metrics; $($rest)*);
    };
    (@__anapao_register_nodes $nodes:ident, $metrics:ident;
        $name:ident : Custom($family:expr); $($rest:tt)*) => {
        $crate::scenario!(@__anapao_register_one_node $nodes, $metrics, $name);
        $crate::scenario!(@__anapao_register_nodes $nodes, $metrics; $($rest)*);
    };
    (@__anapao_register_nodes $nodes:ident, $metrics:ident;
        $name:ident : $family:ident {$($props:tt)*}; $($rest:tt)*) => {
        $crate::scenario!(@__anapao_register_one_node $nodes, $metrics, $name);
        $crate::scenario!(@__anapao_register_nodes $nodes, $metrics; $($rest)*);
    };
    (@__anapao_register_nodes $nodes:ident, $metrics:ident;
        $name:ident : $family:ident; $($rest:tt)*) => {
        $crate::scenario!(@__anapao_register_one_node $nodes, $metrics, $name);
        $crate::scenario!(@__anapao_register_nodes $nodes, $metrics; $($rest)*);
    };
    (@__anapao_register_one_node $nodes:ident, $metrics:ident, $name:ident) => {{
        let __anapao_name = ::std::stringify!($name);
        let __anapao_node_id = $crate::types::NodeId::new(__anapao_name).map_err(|error| {
            $crate::error::SetupError::InvalidParameter {
                name: ::std::format!("nodes.{}.id", __anapao_name),
                reason: ::std::string::ToString::to_string(&error),
            }
        })?;
        let __anapao_metric_id = $crate::types::MetricKey::new(__anapao_name).map_err(|error| {
            $crate::error::SetupError::InvalidParameter {
                name: ::std::format!("metrics.{}.id", __anapao_name),
                reason: ::std::string::ToString::to_string(&error),
            }
        })?;
        $nodes.push((__anapao_name, __anapao_node_id));
        $metrics.push((__anapao_name, __anapao_metric_id));
    }};
    (@__anapao_register_nodes $nodes:ident, $metrics:ident; $($bad:tt)+) => {
        ::std::compile_error!("anapao::scenario!: malformed node declaration")
    };

    (@__anapao_register_edges $edges:ident;) => {};
    (@__anapao_register_edges $edges:ident;
        $name:ident : $from:ident -> $to:ident =>
        $transfer:ident $(($($args:tt)*))? resource {$($props:tt)*}; $($rest:tt)*) => {{
        $crate::scenario!(@__anapao_register_one_edge $edges, $name);
        $crate::scenario!(@__anapao_register_edges $edges; $($rest)*);
    }};
    (@__anapao_register_edges $edges:ident;
        $name:ident : $from:ident -> $to:ident =>
        $transfer:ident $(($($args:tt)*))? state {$($props:tt)*}; $($rest:tt)*) => {{
        $crate::scenario!(@__anapao_register_one_edge $edges, $name);
        $crate::scenario!(@__anapao_register_edges $edges; $($rest)*);
    }};
    (@__anapao_register_edges $edges:ident;
        $name:ident : $from:ident -> $to:ident =>
        $transfer:ident $(($($args:tt)*))?; $($rest:tt)*) => {{
        $crate::scenario!(@__anapao_register_one_edge $edges, $name);
        $crate::scenario!(@__anapao_register_edges $edges; $($rest)*);
    }};
    (@__anapao_register_one_edge $edges:ident, $name:ident) => {{
        let __anapao_name = ::std::stringify!($name);
        let __anapao_id = $crate::types::EdgeId::new(__anapao_name).map_err(|error| {
            $crate::error::SetupError::InvalidParameter {
                name: ::std::format!("edges.{}.id", __anapao_name),
                reason: ::std::string::ToString::to_string(&error),
            }
        })?;
        $edges.push((__anapao_name, __anapao_id));
    }};
    (@__anapao_register_edges $edges:ident; $($bad:tt)+) => {
        ::std::compile_error!("anapao::scenario!: malformed edge declaration")
    };

    // Node declarations and family constructors.
    (@__anapao_build_nodes $builder:ident, $node_id:ident;) => {};
    (@__anapao_build_nodes $builder:ident, $node_id:ident;
        $name:ident : Custom($family:expr) {$($props:tt)*}; $($rest:tt)*) => {{
        let __anapao_family = $family;
        $crate::scenario!(@__anapao_node Custom($name, __anapao_family), $builder, $node_id;
            $($props)*);
        $crate::scenario!(@__anapao_build_nodes $builder, $node_id; $($rest)*);
    }};
    (@__anapao_build_nodes $builder:ident, $node_id:ident;
        $name:ident : Custom($family:expr); $($rest:tt)*) => {{
        let __anapao_family = $family;
        $crate::scenario!(@__anapao_node Custom($name, __anapao_family), $builder, $node_id;);
        $crate::scenario!(@__anapao_build_nodes $builder, $node_id; $($rest)*);
    }};
    (@__anapao_build_nodes $builder:ident, $node_id:ident;
        $name:ident : $family:ident {$($props:tt)*}; $($rest:tt)*) => {{
        $crate::scenario!(@__anapao_node $family($name), $builder, $node_id; $($props)*);
        $crate::scenario!(@__anapao_build_nodes $builder, $node_id; $($rest)*);
    }};
    (@__anapao_build_nodes $builder:ident, $node_id:ident;
        $name:ident : $family:ident; $($rest:tt)*) => {{
        $crate::scenario!(@__anapao_node $family($name), $builder, $node_id;);
        $crate::scenario!(@__anapao_build_nodes $builder, $node_id; $($rest)*);
    }};

    // All node families funnel through one property collector. Config type inference is fixed by
    // the explicit annotation in each configured arm.
    (@__anapao_node Source($name:ident), $builder:ident, $ids:ident; $($props:tt)*) => {
        $crate::scenario!(@__anapao_plain_node source, Source, $name, $builder, $ids; $($props)*)
    };
    (@__anapao_node Process($name:ident), $builder:ident, $ids:ident; $($props:tt)*) => {
        $crate::scenario!(@__anapao_plain_node process, Process, $name, $builder, $ids; $($props)*)
    };
    (@__anapao_node Sink($name:ident), $builder:ident, $ids:ident; $($props:tt)*) => {
        $crate::scenario!(@__anapao_plain_node sink, Sink, $name, $builder, $ids; $($props)*)
    };
    (@__anapao_node Gate($name:ident), $builder:ident, $ids:ident; $($props:tt)*) => {
        $crate::scenario!(@__anapao_plain_node gate, Gate, $name, $builder, $ids; $($props)*)
    };
    (@__anapao_node Custom($name:ident, $family:ident), $builder:ident, $ids:ident; $($props:tt)*) => {{
        let mut __anapao_label: ::std::option::Option<::std::string::String> =
            ::std::option::Option::None;
        let mut __anapao_initial: ::std::option::Option<f64> = ::std::option::Option::None;
        let mut __anapao_tags: ::std::vec::Vec<::std::string::String> = ::std::vec::Vec::new();
        let mut __anapao_metadata: ::std::vec::Vec<(
            ::std::string::String, ::std::string::String
        )> = ::std::vec::Vec::new();
        let mut __anapao_unused = ();
        let mut __anapao_mode: $crate::types::NodeModeConfig =
            ::std::default::Default::default();
        let __anapao_node_name = ::std::stringify!($name);
        $crate::scenario!(@__anapao_node_props Plain, __anapao_unused, __anapao_mode, __anapao_node_name,
            __anapao_label, __anapao_initial, __anapao_tags, __anapao_metadata; [];
            $($props)* ,);
        let mut __anapao_node = $crate::types::ScenarioNode::custom(
            $ids(::std::stringify!($name))?, $family
        );
        $crate::scenario!(@__anapao_apply_common __anapao_node,
            __anapao_label, __anapao_initial, __anapao_tags, __anapao_metadata);
        $builder.insert_node(__anapao_node)?;
    }};
    (@__anapao_plain_node $ctor:ident, $family:ident, $name:ident,
        $builder:ident, $ids:ident; $($props:tt)*) => {{
        let mut __anapao_label: ::std::option::Option<::std::string::String> =
            ::std::option::Option::None;
        let mut __anapao_initial: ::std::option::Option<f64> = ::std::option::Option::None;
        let mut __anapao_tags: ::std::vec::Vec<::std::string::String> = ::std::vec::Vec::new();
        let mut __anapao_metadata: ::std::vec::Vec<(
            ::std::string::String, ::std::string::String
        )> = ::std::vec::Vec::new();
        let mut __anapao_unused = ();
        let mut __anapao_mode: $crate::types::NodeModeConfig =
            ::std::default::Default::default();
        let __anapao_node_name = ::std::stringify!($name);
        $crate::scenario!(@__anapao_node_props Plain, __anapao_unused, __anapao_mode, __anapao_node_name,
            __anapao_label, __anapao_initial, __anapao_tags, __anapao_metadata; [];
            $($props)* ,);
        let mut __anapao_node =
            $crate::types::ScenarioNode::$ctor($ids(::std::stringify!($name))?);
        $crate::scenario!(@__anapao_apply_common __anapao_node,
            __anapao_label, __anapao_initial, __anapao_tags, __anapao_metadata);
        $builder.insert_node(__anapao_node)?;
    }};

    (@__anapao_node Pool($name:ident), $builder:ident, $ids:ident; $($props:tt)*) => {
        $crate::scenario!(@__anapao_config_node PoolConfig, pool, Pool, $name, $builder, $ids; $($props)*)
    };
    (@__anapao_node Drain($name:ident), $builder:ident, $ids:ident; $($props:tt)*) => {
        $crate::scenario!(@__anapao_config_node DrainConfig, drain, Drain, $name, $builder, $ids; $($props)*)
    };
    (@__anapao_node SortingGate($name:ident), $builder:ident, $ids:ident; $($props:tt)*) => {
        $crate::scenario!(@__anapao_config_node SortingGateConfig, sorting_gate, SortingGate, $name, $builder, $ids; $($props)*)
    };
    (@__anapao_node TriggerGate($name:ident), $builder:ident, $ids:ident; $($props:tt)*) => {
        $crate::scenario!(@__anapao_config_node TriggerGateConfig, trigger_gate, TriggerGate, $name, $builder, $ids; $($props)*)
    };
    (@__anapao_node MixedGate($name:ident), $builder:ident, $ids:ident; $($props:tt)*) => {
        $crate::scenario!(@__anapao_config_node MixedGateConfig, mixed_gate, MixedGate, $name, $builder, $ids; $($props)*)
    };
    (@__anapao_node Converter($name:ident), $builder:ident, $ids:ident; $($props:tt)*) => {
        $crate::scenario!(@__anapao_config_node ConverterConfig, converter, Converter, $name, $builder, $ids; $($props)*)
    };
    (@__anapao_node Trader($name:ident), $builder:ident, $ids:ident; $($props:tt)*) => {
        $crate::scenario!(@__anapao_config_node TraderConfig, trader, Trader, $name, $builder, $ids; $($props)*)
    };
    (@__anapao_node Register($name:ident), $builder:ident, $ids:ident; $($props:tt)*) => {
        $crate::scenario!(@__anapao_config_node RegisterConfig, register, Register, $name, $builder, $ids; $($props)*)
    };
    (@__anapao_node Delay($name:ident), $builder:ident, $ids:ident; $($props:tt)*) => {
        $crate::scenario!(@__anapao_config_node DelayConfig, delay, Delay, $name, $builder, $ids; $($props)*)
    };
    (@__anapao_node Queue($name:ident), $builder:ident, $ids:ident; $($props:tt)*) => {
        $crate::scenario!(@__anapao_config_node QueueConfig, queue, Queue, $name, $builder, $ids; $($props)*)
    };
    (@__anapao_node $family:ident($name:ident), $builder:ident, $ids:ident; $($props:tt)*) => {
        ::std::compile_error!("anapao::scenario!: unknown node family")
    };

    (@__anapao_config_node $config_ty:ident, $ctor:ident, $family:ident, $name:ident,
        $builder:ident, $ids:ident; $($props:tt)*) => {{
        let mut __anapao_config: $crate::types::$config_ty = ::std::default::Default::default();
        let mut __anapao_mode: $crate::types::NodeModeConfig =
            ::std::default::Default::default();
        let mut __anapao_label: ::std::option::Option<::std::string::String> =
            ::std::option::Option::None;
        let mut __anapao_initial: ::std::option::Option<f64> = ::std::option::Option::None;
        let mut __anapao_tags: ::std::vec::Vec<::std::string::String> = ::std::vec::Vec::new();
        let mut __anapao_metadata: ::std::vec::Vec<(
            ::std::string::String, ::std::string::String
        )> = ::std::vec::Vec::new();
        let __anapao_node_name = ::std::stringify!($name);
        $crate::scenario!(@__anapao_node_props $family, __anapao_config, __anapao_mode, __anapao_node_name,
            __anapao_label, __anapao_initial, __anapao_tags, __anapao_metadata; [];
            $($props)* ,);
        let mut __anapao_node = $crate::types::ScenarioNode::$ctor(
            $ids(::std::stringify!($name))?, __anapao_config
        );
        $crate::scenario!(@__anapao_apply_common __anapao_node,
            __anapao_label, __anapao_initial, __anapao_tags, __anapao_metadata);
        $builder.insert_node(__anapao_node)?;
    }};

    (@__anapao_apply_common $node:ident, $label:ident, $initial:ident, $tags:ident, $metadata:ident) => {
        if let ::std::option::Option::Some(__anapao_value) = $label {
            $node = $node.with_label(__anapao_value);
        }
        if let ::std::option::Option::Some(__anapao_value) = $initial {
            $node = $node.with_initial_value(__anapao_value);
        }
        for __anapao_value in $tags {
            $node = $node.with_tag(__anapao_value);
        }
        for (__anapao_key, __anapao_value) in $metadata {
            $node = $node.with_metadata(__anapao_key, __anapao_value);
        }
    };

    // Common node properties.
    (@__anapao_node_props $family:ident, $config:ident, $mode:ident, $node_name:ident, $label:ident, $initial:ident,
        $tags:ident, $metadata:ident; [$($seen:ident)*];) => {};
    (@__anapao_node_props $family:ident, $config:ident, $mode:ident, $node_name:ident, $label:ident, $initial:ident,
        $tags:ident, $metadata:ident; [$($seen:ident)*]; , $($rest:tt)*) => {
        $crate::scenario!(@__anapao_node_props $family, $config, $mode, $node_name, $label, $initial,
            $tags, $metadata; [$($seen)*]; $($rest)*);
    };
    (@__anapao_node_props $family:ident, $config:ident, $mode:ident, $node_name:ident, $label:ident, $initial:ident,
        $tags:ident, $metadata:ident; [$($seen:ident)*]; label: $value:expr, $($rest:tt)*) => {
        $crate::scenario!(@__anapao_assert_label [$($seen)*]);
        let __anapao_value = $value;
        $label = ::std::option::Option::Some(::std::convert::Into::into(__anapao_value));
        $crate::scenario!(@__anapao_node_props $family, $config, $mode, $node_name, $label, $initial,
            $tags, $metadata; [$($seen)* label]; $($rest)*);
    };
    (@__anapao_node_props $family:ident, $config:ident, $mode:ident, $node_name:ident, $label:ident, $initial:ident,
        $tags:ident, $metadata:ident; [$($seen:ident)*]; initial: $value:expr, $($rest:tt)*) => {
        $crate::scenario!(@__anapao_assert_initial [$($seen)*]);
        let __anapao_value = $value;
        $initial = ::std::option::Option::Some(__anapao_value);
        $crate::scenario!(@__anapao_node_props $family, $config, $mode, $node_name, $label, $initial,
            $tags, $metadata; [$($seen)* initial]; $($rest)*);
    };
    (@__anapao_node_props $family:ident, $config:ident, $mode:ident, $node_name:ident, $label:ident, $initial:ident,
        $tags:ident, $metadata:ident; [$($seen:ident)*]; tags [$($value:expr),* $(,)?], $($rest:tt)*) => {
        $crate::scenario!(@__anapao_assert_tags [$($seen)*]);
        $(
            let __anapao_value = $value;
            $tags.push(::std::convert::Into::into(__anapao_value));
        )*
        $crate::scenario!(@__anapao_node_props $family, $config, $mode, $node_name, $label, $initial,
            $tags, $metadata; [$($seen)* tags]; $($rest)*);
    };
    (@__anapao_node_props $family:ident, $config:ident, $mode:ident, $node_name:ident, $label:ident, $initial:ident,
        $tags:ident, $metadata:ident; [$($seen:ident)*]; metadata {$($key:expr => $value:expr),* $(,)?}, $($rest:tt)*) => {
        $crate::scenario!(@__anapao_assert_metadata [$($seen)*]);
        $(
            let __anapao_key = $key;
            let __anapao_value = $value;
            $metadata.push((::std::convert::Into::into(__anapao_key),
                ::std::convert::Into::into(__anapao_value)));
        )*
        $crate::scenario!(@__anapao_node_props $family, $config, $mode, $node_name, $label, $initial,
            $tags, $metadata; [$($seen)* metadata]; $($rest)*);
    };

    // Typed config is exclusive with every native config property.
    (@__anapao_node_props $family:ident, $config:ident, $mode:ident, $node_name:ident, $label:ident, $initial:ident,
        $tags:ident, $metadata:ident; [$($seen:ident)*]; config: $value:expr, $($rest:tt)*) => {
        $crate::scenario!(@__anapao_family_config $family);
        $crate::scenario!(@__anapao_assert_config [$($seen)*]);
        $crate::scenario!(@__anapao_assert_no_native_config [$($seen)*]);
        let __anapao_value = $value;
        $config = __anapao_value;
        $crate::scenario!(@__anapao_node_props $family, $config, $mode, $node_name, $label, $initial,
            $tags, $metadata; [$($seen)* config]; $($rest)*);
    };

    // Pool.
    (@__anapao_node_props Pool, $config:ident, $mode:ident, $node_name:ident, $label:ident, $initial:ident,
        $tags:ident, $metadata:ident; [$($seen:ident)*]; capacity: none, $($rest:tt)*) => {
        $crate::scenario!(@__anapao_native_guard capacity [$($seen)*]);
        $config = $config.without_capacity();
        $crate::scenario!(@__anapao_node_props Pool, $config, $mode, $node_name, $label, $initial,
            $tags, $metadata; [$($seen)* capacity]; $($rest)*);
    };
    (@__anapao_node_props Pool, $config:ident, $mode:ident, $node_name:ident, $label:ident, $initial:ident,
        $tags:ident, $metadata:ident; [$($seen:ident)*]; capacity: $value:expr, $($rest:tt)*) => {
        $crate::scenario!(@__anapao_native_guard capacity [$($seen)*]);
        let __anapao_value = $value;
        $config = $config.with_capacity(__anapao_value);
        $crate::scenario!(@__anapao_node_props Pool, $config, $mode, $node_name, $label, $initial,
            $tags, $metadata; [$($seen)* capacity]; $($rest)*);
    };
    (@__anapao_node_props Pool, $config:ident, $mode:ident, $node_name:ident, $label:ident, $initial:ident,
        $tags:ident, $metadata:ident; [$($seen:ident)*]; allow_negative_start: $value:expr, $($rest:tt)*) => {
        $crate::scenario!(@__anapao_native_guard allow_negative_start [$($seen)*]);
        let __anapao_value = $value;
        $config = $config.with_allow_negative_start(__anapao_value);
        $crate::scenario!(@__anapao_node_props Pool, $config, $mode, $node_name, $label, $initial,
            $tags, $metadata; [$($seen)* allow_negative_start]; $($rest)*);
    };

    // Converter/trader and register fields.
    (@__anapao_node_props $family:ident, $config:ident, $mode:ident, $node_name:ident, $label:ident, $initial:ident,
        $tags:ident, $metadata:ident; [$($seen:ident)*]; ignore_disabled_inputs: $value:expr, $($rest:tt)*) => {
        $crate::scenario!(@__anapao_family_converter_trader $family);
        $crate::scenario!(@__anapao_native_guard ignore_disabled_inputs [$($seen)*]);
        let __anapao_value = $value;
        $config = $config.with_ignore_disabled_inputs(__anapao_value);
        $crate::scenario!(@__anapao_node_props $family, $config, $mode, $node_name, $label, $initial,
            $tags, $metadata; [$($seen)* ignore_disabled_inputs]; $($rest)*);
    };
    (@__anapao_node_props Register, $config:ident, $mode:ident, $node_name:ident, $label:ident, $initial:ident,
        $tags:ident, $metadata:ident; [$($seen:ident)*]; interactive: $value:expr, $($rest:tt)*) => {
        $crate::scenario!(@__anapao_native_guard interactive [$($seen)*]);
        let __anapao_value = $value;
        $config = $config.with_interactive(__anapao_value);
        $crate::scenario!(@__anapao_node_props Register, $config, $mode, $node_name, $label, $initial,
            $tags, $metadata; [$($seen)* interactive]; $($rest)*);
    };
    (@__anapao_node_props Register, $config:ident, $mode:ident, $node_name:ident, $label:ident, $initial:ident,
        $tags:ident, $metadata:ident; [$($seen:ident)*]; min_value: none, $($rest:tt)*) => {
        $crate::scenario!(@__anapao_native_guard min_value [$($seen)*]);
        $config = $config.without_min_value();
        $crate::scenario!(@__anapao_node_props Register, $config, $mode, $node_name, $label, $initial,
            $tags, $metadata; [$($seen)* min_value]; $($rest)*);
    };
    (@__anapao_node_props Register, $config:ident, $mode:ident, $node_name:ident, $label:ident, $initial:ident,
        $tags:ident, $metadata:ident; [$($seen:ident)*]; min_value: $value:expr, $($rest:tt)*) => {
        $crate::scenario!(@__anapao_native_guard min_value [$($seen)*]);
        let __anapao_value = $value;
        $config = $config.with_min_value(__anapao_value);
        $crate::scenario!(@__anapao_node_props Register, $config, $mode, $node_name, $label, $initial,
            $tags, $metadata; [$($seen)* min_value]; $($rest)*);
    };
    (@__anapao_node_props Register, $config:ident, $mode:ident, $node_name:ident, $label:ident, $initial:ident,
        $tags:ident, $metadata:ident; [$($seen:ident)*]; max_value: none, $($rest:tt)*) => {
        $crate::scenario!(@__anapao_native_guard max_value [$($seen)*]);
        $config = $config.without_max_value();
        $crate::scenario!(@__anapao_node_props Register, $config, $mode, $node_name, $label, $initial,
            $tags, $metadata; [$($seen)* max_value]; $($rest)*);
    };
    (@__anapao_node_props Register, $config:ident, $mode:ident, $node_name:ident, $label:ident, $initial:ident,
        $tags:ident, $metadata:ident; [$($seen:ident)*]; max_value: $value:expr, $($rest:tt)*) => {
        $crate::scenario!(@__anapao_native_guard max_value [$($seen)*]);
        let __anapao_value = $value;
        $config = $config.with_max_value(__anapao_value);
        $crate::scenario!(@__anapao_node_props Register, $config, $mode, $node_name, $label, $initial,
            $tags, $metadata; [$($seen)* max_value]; $($rest)*);
    };

    // Positive Delay/Queue fields return the established SetupError paths without unchecked
    // extraction.
    (@__anapao_node_props Delay, $config:ident, $mode:ident, $node_name:ident, $label:ident, $initial:ident,
        $tags:ident, $metadata:ident; [$($seen:ident)*]; steps: $value:expr, $($rest:tt)*) => {
        $crate::scenario!(@__anapao_native_guard steps [$($seen)*]);
        let __anapao_value = $value;
        let __anapao_value = ::std::num::NonZeroU64::new(__anapao_value).ok_or_else(||
            $crate::error::SetupError::InvalidParameter {
                name: ::std::format!("nodes.{}.config.delay_steps", $node_name),
                reason: ::std::convert::From::from("must be greater than 0"),
            })?;
        $config = $config.with_delay_steps(__anapao_value);
        $crate::scenario!(@__anapao_node_props Delay, $config, $mode, $node_name, $label, $initial,
            $tags, $metadata; [$($seen)* steps]; $($rest)*);
    };
    (@__anapao_node_props Queue, $config:ident, $mode:ident, $node_name:ident, $label:ident, $initial:ident,
        $tags:ident, $metadata:ident; [$($seen:ident)*]; capacity: none, $($rest:tt)*) => {
        $crate::scenario!(@__anapao_native_guard capacity [$($seen)*]);
        $config = $config.without_capacity();
        $crate::scenario!(@__anapao_node_props Queue, $config, $mode, $node_name, $label, $initial,
            $tags, $metadata; [$($seen)* capacity]; $($rest)*);
    };
    (@__anapao_node_props Queue, $config:ident, $mode:ident, $node_name:ident, $label:ident, $initial:ident,
        $tags:ident, $metadata:ident; [$($seen:ident)*]; capacity: $value:expr, $($rest:tt)*) => {
        $crate::scenario!(@__anapao_native_guard capacity [$($seen)*]);
        let __anapao_value = $value;
        let __anapao_value = ::std::num::NonZeroU64::new(__anapao_value).ok_or_else(||
            $crate::error::SetupError::InvalidParameter {
                name: ::std::format!("nodes.{}.config.capacity", $node_name),
                reason: ::std::convert::From::from("must be greater than 0"),
            })?;
        $config = $config.with_capacity(__anapao_value);
        $crate::scenario!(@__anapao_node_props Queue, $config, $mode, $node_name, $label, $initial,
            $tags, $metadata; [$($seen)* capacity]; $($rest)*);
    };
    (@__anapao_node_props Queue, $config:ident, $mode:ident, $node_name:ident, $label:ident, $initial:ident,
        $tags:ident, $metadata:ident; [$($seen:ident)*]; release_per_step: $value:expr, $($rest:tt)*) => {
        $crate::scenario!(@__anapao_native_guard release_per_step [$($seen)*]);
        let __anapao_value = $value;
        let __anapao_value = ::std::num::NonZeroU64::new(__anapao_value).ok_or_else(||
            $crate::error::SetupError::InvalidParameter {
                name: ::std::format!("nodes.{}.config.release_per_step", $node_name),
                reason: ::std::convert::From::from("must be greater than 0"),
            })?;
        $config = $config.with_release_per_step(__anapao_value);
        $crate::scenario!(@__anapao_node_props Queue, $config, $mode, $node_name, $label, $initial,
            $tags, $metadata; [$($seen)* release_per_step]; $($rest)*);
    };

    // Mode applies to all configurable mode families except Register. Trigger/action are the
    // field-level shorthand for constructing NodeModeConfig.
    (@__anapao_node_props $family:ident, $config:ident, $mode:ident, $node_name:ident, $label:ident, $initial:ident,
        $tags:ident, $metadata:ident; [$($seen:ident)*]; mode: $value:expr, $($rest:tt)*) => {
        $crate::scenario!(@__anapao_family_mode $family);
        $crate::scenario!(@__anapao_native_guard mode [$($seen)*]);
        $crate::scenario!(@__anapao_assert_trigger [$($seen)*]);
        $crate::scenario!(@__anapao_assert_action [$($seen)*]);
        let __anapao_value = $value;
        $mode = __anapao_value;
        $config = $config.with_mode(::std::clone::Clone::clone(&$mode));
        $crate::scenario!(@__anapao_node_props $family, $config, $mode, $node_name, $label, $initial,
            $tags, $metadata; [$($seen)* mode]; $($rest)*);
    };
    (@__anapao_node_props $family:ident, $config:ident, $mode:ident, $node_name:ident, $label:ident, $initial:ident,
        $tags:ident, $metadata:ident; [$($seen:ident)*]; trigger: $value:expr, $($rest:tt)*) => {
        $crate::scenario!(@__anapao_family_mode $family);
        $crate::scenario!(@__anapao_native_guard trigger [$($seen)*]);
        $crate::scenario!(@__anapao_assert_mode [$($seen)*]);
        let __anapao_value = $value;
        $mode.trigger_mode = __anapao_value;
        $config = $config.with_mode(::std::clone::Clone::clone(&$mode));
        $crate::scenario!(@__anapao_node_props $family, $config, $mode, $node_name, $label, $initial,
            $tags, $metadata; [$($seen)* trigger]; $($rest)*);
    };
    (@__anapao_node_props $family:ident, $config:ident, $mode:ident, $node_name:ident, $label:ident, $initial:ident,
        $tags:ident, $metadata:ident; [$($seen:ident)*]; action: $value:expr, $($rest:tt)*) => {
        $crate::scenario!(@__anapao_family_mode $family);
        $crate::scenario!(@__anapao_native_guard action [$($seen)*]);
        $crate::scenario!(@__anapao_assert_mode [$($seen)*]);
        let __anapao_value = $value;
        $mode.action_mode = __anapao_value;
        $config = $config.with_mode(::std::clone::Clone::clone(&$mode));
        $crate::scenario!(@__anapao_node_props $family, $config, $mode, $node_name, $label, $initial,
            $tags, $metadata; [$($seen)* action]; $($rest)*);
    };
    (@__anapao_node_props $family:ident, $config:ident, $mode:ident, $node_name:ident, $label:ident, $initial:ident,
        $tags:ident, $metadata:ident; [$($seen:ident)*]; $($bad:tt)+) => {
        ::std::compile_error!("anapao::scenario!: unknown, duplicate, or wrong-family node property")
    };

    (@__anapao_family_converter_trader Converter) => {};
    (@__anapao_family_converter_trader Trader) => {};
    (@__anapao_family_converter_trader $other:ident) => {
        ::std::compile_error!("anapao::scenario!: ignore_disabled_inputs is only valid for Converter or Trader")
    };
    (@__anapao_family_config Pool) => {};
    (@__anapao_family_config Drain) => {};
    (@__anapao_family_config SortingGate) => {};
    (@__anapao_family_config TriggerGate) => {};
    (@__anapao_family_config MixedGate) => {};
    (@__anapao_family_config Converter) => {};
    (@__anapao_family_config Trader) => {};
    (@__anapao_family_config Register) => {};
    (@__anapao_family_config Delay) => {};
    (@__anapao_family_config Queue) => {};
    (@__anapao_family_config $other:ident) => {
        ::std::compile_error!("anapao::scenario!: typed config is only valid for configured node families")
    };
    (@__anapao_family_mode Pool) => {};
    (@__anapao_family_mode Drain) => {};
    (@__anapao_family_mode SortingGate) => {};
    (@__anapao_family_mode TriggerGate) => {};
    (@__anapao_family_mode MixedGate) => {};
    (@__anapao_family_mode Converter) => {};
    (@__anapao_family_mode Trader) => {};
    (@__anapao_family_mode Delay) => {};
    (@__anapao_family_mode Queue) => {};
    (@__anapao_family_mode $other:ident) => {
        ::std::compile_error!("anapao::scenario!: mode is not valid for this node family")
    };

    // Duplicate/exclusivity checks. Native fields all share the config exclusion check, while
    // the per-field scanner emits compile-time diagnostics for duplicates.
    (@__anapao_native_guard $field:ident [$($seen:ident)*]) => {
        $crate::scenario!(@__anapao_assert_config [$($seen)*]);
        $crate::scenario!(@__anapao_assert_field $field [$($seen)*]);
    };
    (@__anapao_assert_no_native_config []) => {};
    (@__anapao_assert_no_native_config [label $($rest:ident)*]) => {
        $crate::scenario!(@__anapao_assert_no_native_config [$($rest)*])
    };
    (@__anapao_assert_no_native_config [initial $($rest:ident)*]) => {
        $crate::scenario!(@__anapao_assert_no_native_config [$($rest)*])
    };
    (@__anapao_assert_no_native_config [tags $($rest:ident)*]) => {
        $crate::scenario!(@__anapao_assert_no_native_config [$($rest)*])
    };
    (@__anapao_assert_no_native_config [metadata $($rest:ident)*]) => {
        $crate::scenario!(@__anapao_assert_no_native_config [$($rest)*])
    };
    (@__anapao_assert_no_native_config [$other:ident $($rest:ident)*]) => {
        ::std::compile_error!("anapao::scenario!: typed config cannot be mixed with native config fields")
    };

    // A compact token equality dispatcher avoids a separate scanner arm set for every field.
    (@__anapao_assert_field $field:ident []) => {};
    (@__anapao_assert_field $field:ident [$head:ident $($rest:ident)*]) => {
        $crate::scenario!(@__anapao_compare_field $field $head; [$($rest)*]);
    };
    (@__anapao_compare_field label label; [$($rest:ident)*]) => { ::std::compile_error!("anapao::scenario!: duplicate label property") };
    (@__anapao_compare_field initial initial; [$($rest:ident)*]) => { ::std::compile_error!("anapao::scenario!: duplicate initial property") };
    (@__anapao_compare_field tags tags; [$($rest:ident)*]) => { ::std::compile_error!("anapao::scenario!: duplicate tags property") };
    (@__anapao_compare_field metadata metadata; [$($rest:ident)*]) => { ::std::compile_error!("anapao::scenario!: duplicate metadata property") };
    (@__anapao_compare_field config config; [$($rest:ident)*]) => { ::std::compile_error!("anapao::scenario!: duplicate config property") };
    (@__anapao_compare_field capacity capacity; [$($rest:ident)*]) => { ::std::compile_error!("anapao::scenario!: duplicate capacity property") };
    (@__anapao_compare_field allow_negative_start allow_negative_start; [$($rest:ident)*]) => { ::std::compile_error!("anapao::scenario!: duplicate allow_negative_start property") };
    (@__anapao_compare_field ignore_disabled_inputs ignore_disabled_inputs; [$($rest:ident)*]) => { ::std::compile_error!("anapao::scenario!: duplicate ignore_disabled_inputs property") };
    (@__anapao_compare_field interactive interactive; [$($rest:ident)*]) => { ::std::compile_error!("anapao::scenario!: duplicate interactive property") };
    (@__anapao_compare_field min_value min_value; [$($rest:ident)*]) => { ::std::compile_error!("anapao::scenario!: duplicate min_value property") };
    (@__anapao_compare_field max_value max_value; [$($rest:ident)*]) => { ::std::compile_error!("anapao::scenario!: duplicate max_value property") };
    (@__anapao_compare_field steps steps; [$($rest:ident)*]) => { ::std::compile_error!("anapao::scenario!: duplicate steps property") };
    (@__anapao_compare_field release_per_step release_per_step; [$($rest:ident)*]) => { ::std::compile_error!("anapao::scenario!: duplicate release_per_step property") };
    (@__anapao_compare_field mode mode; [$($rest:ident)*]) => { ::std::compile_error!("anapao::scenario!: duplicate mode property") };
    (@__anapao_compare_field trigger trigger; [$($rest:ident)*]) => { ::std::compile_error!("anapao::scenario!: duplicate trigger property") };
    (@__anapao_compare_field action action; [$($rest:ident)*]) => { ::std::compile_error!("anapao::scenario!: duplicate action property") };
    (@__anapao_compare_field connection connection; [$($rest:ident)*]) => { ::std::compile_error!("anapao::scenario!: duplicate connection property") };
    (@__anapao_compare_field token_size token_size; [$($rest:ident)*]) => { ::std::compile_error!("anapao::scenario!: duplicate token_size property") };
    (@__anapao_compare_field enabled enabled; [$($rest:ident)*]) => { ::std::compile_error!("anapao::scenario!: duplicate enabled property") };
    (@__anapao_compare_field role role; [$($rest:ident)*]) => { ::std::compile_error!("anapao::scenario!: duplicate role property") };
    (@__anapao_compare_field formula formula; [$($rest:ident)*]) => { ::std::compile_error!("anapao::scenario!: duplicate formula property") };
    (@__anapao_compare_field target target; [$($rest:ident)*]) => { ::std::compile_error!("anapao::scenario!: duplicate target property") };
    (@__anapao_compare_field resource_filter resource_filter; [$($rest:ident)*]) => { ::std::compile_error!("anapao::scenario!: duplicate resource_filter property") };
    (@__anapao_compare_field $field:ident $head:ident; [$($rest:ident)*]) => {
        $crate::scenario!(@__anapao_assert_field $field [$($rest)*])
    };
    (@__anapao_assert_label [$($seen:ident)*]) => { $crate::scenario!(@__anapao_assert_field label [$($seen)*]) };
    (@__anapao_assert_initial [$($seen:ident)*]) => { $crate::scenario!(@__anapao_assert_field initial [$($seen)*]) };
    (@__anapao_assert_tags [$($seen:ident)*]) => { $crate::scenario!(@__anapao_assert_field tags [$($seen)*]) };
    (@__anapao_assert_metadata [$($seen:ident)*]) => { $crate::scenario!(@__anapao_assert_field metadata [$($seen)*]) };
    (@__anapao_assert_config [$($seen:ident)*]) => { $crate::scenario!(@__anapao_assert_field config [$($seen)*]) };
    (@__anapao_assert_mode [$($seen:ident)*]) => { $crate::scenario!(@__anapao_assert_field mode [$($seen)*]) };
    (@__anapao_assert_trigger [$($seen:ident)*]) => { $crate::scenario!(@__anapao_assert_field trigger [$($seen)*]) };
    (@__anapao_assert_action [$($seen:ident)*]) => { $crate::scenario!(@__anapao_assert_field action [$($seen)*]) };

    // Edge declarations. A connection block is an optional suffix after the transfer.
    (@__anapao_build_edges $builder:ident, $nodes:ident, $metrics:ident, $edges:ident;) => {};
    (@__anapao_build_edges $builder:ident, $nodes:ident, $metrics:ident, $edges:ident;
        $name:ident : $from:ident -> $to:ident =>
        $transfer:ident $(($($args:tt)*))? resource {$($props:tt)*}; $($rest:tt)*) => {{
        $crate::scenario!(@__anapao_edge_transfer $builder, $nodes, $metrics, $edges,
            $name, $from, $to; $transfer $(($($args)*))? resource {$($props)*});
        $crate::scenario!(@__anapao_build_edges $builder, $nodes, $metrics, $edges; $($rest)*);
    }};
    (@__anapao_build_edges $builder:ident, $nodes:ident, $metrics:ident, $edges:ident;
        $name:ident : $from:ident -> $to:ident =>
        $transfer:ident $(($($args:tt)*))? state {$($props:tt)*}; $($rest:tt)*) => {{
        $crate::scenario!(@__anapao_edge_transfer $builder, $nodes, $metrics, $edges,
            $name, $from, $to; $transfer $(($($args)*))? state {$($props)*});
        $crate::scenario!(@__anapao_build_edges $builder, $nodes, $metrics, $edges; $($rest)*);
    }};
    (@__anapao_build_edges $builder:ident, $nodes:ident, $metrics:ident, $edges:ident;
        $name:ident : $from:ident -> $to:ident =>
        $transfer:ident $(($($args:tt)*))?; $($rest:tt)*) => {{
        $crate::scenario!(@__anapao_edge_transfer $builder, $nodes, $metrics, $edges,
            $name, $from, $to; $transfer $(($($args)*))?);
        $crate::scenario!(@__anapao_build_edges $builder, $nodes, $metrics, $edges; $($rest)*);
    }};

    (@__anapao_edge_transfer $builder:ident, $nodes:ident, $metrics:ident, $edges:ident,
        $name:ident, $from:ident, $to:ident; fixed($value:expr) $($suffix:tt)*) => {{
        let __anapao_value = $value;
        let __anapao_transfer = $crate::types::TransferSpec::Fixed { amount: __anapao_value };
        $crate::scenario!(@__anapao_edge_finish $builder, $nodes, $edges,
            $name, $from, $to, __anapao_transfer; $($suffix)*);
    }};
    (@__anapao_edge_transfer $builder:ident, $nodes:ident, $metrics:ident, $edges:ident,
        $name:ident, $from:ident, $to:ident; fraction($numerator:expr, $denominator:expr) $($suffix:tt)*) => {{
        let __anapao_numerator = $numerator;
        let __anapao_denominator = $denominator;
        let __anapao_transfer = $crate::types::TransferSpec::Fraction {
            numerator: __anapao_numerator, denominator: __anapao_denominator
        };
        $crate::scenario!(@__anapao_edge_finish $builder, $nodes, $edges,
            $name, $from, $to, __anapao_transfer; $($suffix)*);
    }};
    (@__anapao_edge_transfer $builder:ident, $nodes:ident, $metrics:ident, $edges:ident,
        $name:ident, $from:ident, $to:ident; remaining $($suffix:tt)*) => {{
        let __anapao_transfer = $crate::types::TransferSpec::Remaining;
        $crate::scenario!(@__anapao_edge_finish $builder, $nodes, $edges,
            $name, $from, $to, __anapao_transfer; $($suffix)*);
    }};
    (@__anapao_edge_transfer $builder:ident, $nodes:ident, $metrics:ident, $edges:ident,
        $name:ident, $from:ident, $to:ident; metric_scaled($metric:ident, $factor:expr) $($suffix:tt)*) => {{
        let __anapao_factor = $factor;
        let __anapao_transfer = $crate::types::TransferSpec::MetricScaled {
            metric: $metrics(::std::stringify!($metric))?, factor: __anapao_factor
        };
        $crate::scenario!(@__anapao_edge_finish $builder, $nodes, $edges,
            $name, $from, $to, __anapao_transfer; $($suffix)*);
    }};
    (@__anapao_edge_transfer $builder:ident, $nodes:ident, $metrics:ident, $edges:ident,
        $name:ident, $from:ident, $to:ident; expression($formula:expr) $($suffix:tt)*) => {{
        let __anapao_formula = $formula;
        let __anapao_transfer = $crate::types::TransferSpec::Expression {
            formula: ::std::convert::Into::into(__anapao_formula)
        };
        $crate::scenario!(@__anapao_edge_finish $builder, $nodes, $edges,
            $name, $from, $to, __anapao_transfer; $($suffix)*);
    }};
    (@__anapao_edge_transfer $builder:ident, $nodes:ident, $metrics:ident, $edges:ident,
        $name:ident, $from:ident, $to:ident; transfer($transfer:expr) $($suffix:tt)*) => {{
        let __anapao_transfer = $transfer;
        $crate::scenario!(@__anapao_edge_finish $builder, $nodes, $edges,
            $name, $from, $to, __anapao_transfer; $($suffix)*);
    }};
    (@__anapao_edge_transfer $($bad:tt)*) => {
        ::std::compile_error!("anapao::scenario!: malformed transfer")
    };

    (@__anapao_edge_finish $builder:ident, $nodes:ident, $edges:ident,
        $name:ident, $from:ident, $to:ident, $transfer:ident;) => {{
        let __anapao_edge = $crate::types::ScenarioEdge::resource(
            $edges(::std::stringify!($name))?, $nodes(::std::stringify!($from))?,
            $nodes(::std::stringify!($to))?, $transfer,
            <$crate::types::ResourceConnection as ::std::default::Default>::default()
        );
        $builder.insert_edge(__anapao_edge)?;
    }};
    (@__anapao_edge_finish $builder:ident, $nodes:ident, $edges:ident,
        $name:ident, $from:ident, $to:ident, $transfer:ident; resource {$($props:tt)*}) => {{
        let mut __anapao_connection =
            <$crate::types::ResourceConnection as ::std::default::Default>::default();
        let mut __anapao_enabled: bool = true;
        let mut __anapao_metadata: ::std::vec::Vec<(
            ::std::string::String, ::std::string::String
        )> = ::std::vec::Vec::new();
        let __anapao_edge_name = ::std::stringify!($name);
        $crate::scenario!(@__anapao_resource_props __anapao_connection, __anapao_edge_name,
            __anapao_enabled, __anapao_metadata; []; $($props)* ,);
        let mut __anapao_edge = $crate::types::ScenarioEdge::resource(
            $edges(::std::stringify!($name))?, $nodes(::std::stringify!($from))?,
            $nodes(::std::stringify!($to))?, $transfer, __anapao_connection
        ).with_enabled(__anapao_enabled);
        for (__anapao_key, __anapao_value) in __anapao_metadata {
            __anapao_edge = __anapao_edge.with_metadata(__anapao_key, __anapao_value);
        }
        $builder.insert_edge(__anapao_edge)?;
    }};
    (@__anapao_edge_finish $builder:ident, $nodes:ident, $edges:ident,
        $name:ident, $from:ident, $to:ident, $transfer:ident; state {$($props:tt)*}) => {{
        let mut __anapao_connection =
            <$crate::types::StateConnection as ::std::default::Default>::default();
        let mut __anapao_enabled: bool = true;
        let mut __anapao_metadata: ::std::vec::Vec<(
            ::std::string::String, ::std::string::String
        )> = ::std::vec::Vec::new();
        $crate::scenario!(@__anapao_state_props __anapao_connection,
            __anapao_enabled, __anapao_metadata, $edges; []; $($props)* ,);
        let mut __anapao_edge = $crate::types::ScenarioEdge::state(
            $edges(::std::stringify!($name))?, $nodes(::std::stringify!($from))?,
            $nodes(::std::stringify!($to))?, $transfer, __anapao_connection
        ).with_enabled(__anapao_enabled);
        for (__anapao_key, __anapao_value) in __anapao_metadata {
            __anapao_edge = __anapao_edge.with_metadata(__anapao_key, __anapao_value);
        }
        $builder.insert_edge(__anapao_edge)?;
    }};
    (@__anapao_edge_finish $($bad:tt)*) => {
        ::std::compile_error!("anapao::scenario!: malformed connection suffix")
    };

    // Resource connection fields.
    (@__anapao_resource_props $connection:ident, $edge_name:ident, $enabled:ident, $metadata:ident;
        [$($seen:ident)*];) => {};
    (@__anapao_resource_props $connection:ident, $edge_name:ident, $enabled:ident, $metadata:ident;
        [$($seen:ident)*]; , $($rest:tt)*) => {
        $crate::scenario!(@__anapao_resource_props $connection, $edge_name, $enabled, $metadata;
            [$($seen)*]; $($rest)*);
    };
    (@__anapao_resource_props $connection:ident, $edge_name:ident, $enabled:ident, $metadata:ident;
        [$($seen:ident)*]; connection: $value:expr, $($rest:tt)*) => {
        $crate::scenario!(@__anapao_assert_field connection [$($seen)*]);
        $crate::scenario!(@__anapao_assert_field token_size [$($seen)*]);
        let __anapao_value = $value;
        $connection = __anapao_value;
        $crate::scenario!(@__anapao_resource_props $connection, $edge_name, $enabled, $metadata;
            [$($seen)* connection]; $($rest)*);
    };
    (@__anapao_resource_props $connection:ident, $edge_name:ident, $enabled:ident, $metadata:ident;
        [$($seen:ident)*]; token_size: $value:expr, $($rest:tt)*) => {
        $crate::scenario!(@__anapao_assert_field token_size [$($seen)*]);
        $crate::scenario!(@__anapao_assert_field connection [$($seen)*]);
        let __anapao_value = $value;
        let __anapao_value = ::std::num::NonZeroU64::new(__anapao_value).ok_or_else(||
            $crate::error::SetupError::InvalidParameter {
                name: ::std::format!(
                    "edges.{}.connection.resource.token_size", $edge_name
                ),
                reason: ::std::convert::From::from("must be greater than 0"),
            })?;
        $connection = $connection.with_token_size(__anapao_value);
        $crate::scenario!(@__anapao_resource_props $connection, $edge_name, $enabled, $metadata;
            [$($seen)* token_size]; $($rest)*);
    };
    (@__anapao_resource_props $connection:ident, $edge_name:ident, $enabled:ident, $metadata:ident;
        [$($seen:ident)*]; enabled: $value:expr, $($rest:tt)*) => {
        $crate::scenario!(@__anapao_assert_field enabled [$($seen)*]);
        let __anapao_value = $value;
        $enabled = __anapao_value;
        $crate::scenario!(@__anapao_resource_props $connection, $edge_name, $enabled, $metadata;
            [$($seen)* enabled]; $($rest)*);
    };
    (@__anapao_resource_props $connection:ident, $edge_name:ident, $enabled:ident, $metadata:ident;
        [$($seen:ident)*]; metadata {$($key:expr => $value:expr),* $(,)?}, $($rest:tt)*) => {
        $crate::scenario!(@__anapao_assert_field metadata [$($seen)*]);
        $(
            let __anapao_key = $key;
            let __anapao_value = $value;
            $metadata.push((::std::convert::Into::into(__anapao_key),
                ::std::convert::Into::into(__anapao_value)));
        )*
        $crate::scenario!(@__anapao_resource_props $connection, $edge_name, $enabled, $metadata;
            [$($seen)* metadata]; $($rest)*);
    };
    (@__anapao_resource_props $($bad:tt)*) => {
        ::std::compile_error!("anapao::scenario!: unknown or duplicate resource connection property")
    };

    // State connection fields and targets.
    (@__anapao_state_props $connection:ident, $enabled:ident, $metadata:ident, $edges:ident;
        [$($seen:ident)*];) => {};
    (@__anapao_state_props $connection:ident, $enabled:ident, $metadata:ident, $edges:ident;
        [$($seen:ident)*]; , $($rest:tt)*) => {
        $crate::scenario!(@__anapao_state_props $connection, $enabled, $metadata, $edges;
            [$($seen)*]; $($rest)*);
    };
    (@__anapao_state_props $connection:ident, $enabled:ident, $metadata:ident, $edges:ident;
        [$($seen:ident)*]; connection: $value:expr, $($rest:tt)*) => {
        $crate::scenario!(@__anapao_assert_field connection [$($seen)*]);
        $crate::scenario!(@__anapao_assert_no_native_connection [$($seen)*]);
        let __anapao_value = $value;
        $connection = __anapao_value;
        $crate::scenario!(@__anapao_state_props $connection, $enabled, $metadata, $edges;
            [$($seen)* connection]; $($rest)*);
    };
    (@__anapao_state_props $connection:ident, $enabled:ident, $metadata:ident, $edges:ident;
        [$($seen:ident)*]; role: $value:expr, $($rest:tt)*) => {
        $crate::scenario!(@__anapao_state_native_guard role [$($seen)*]);
        let __anapao_value = $value;
        $connection = $connection.with_role(__anapao_value);
        $crate::scenario!(@__anapao_state_props $connection, $enabled, $metadata, $edges;
            [$($seen)* role]; $($rest)*);
    };
    (@__anapao_state_props $connection:ident, $enabled:ident, $metadata:ident, $edges:ident;
        [$($seen:ident)*]; formula: $value:expr, $($rest:tt)*) => {
        $crate::scenario!(@__anapao_state_native_guard formula [$($seen)*]);
        let __anapao_value = $value;
        $connection = $connection.with_formula(__anapao_value);
        $crate::scenario!(@__anapao_state_props $connection, $enabled, $metadata, $edges;
            [$($seen)* formula]; $($rest)*);
    };
    (@__anapao_state_props $connection:ident, $enabled:ident, $metadata:ident, $edges:ident;
        [$($seen:ident)*]; target: node, $($rest:tt)*) => {
        $crate::scenario!(@__anapao_state_native_guard target [$($seen)*]);
        $connection = $connection.with_target($crate::types::StateTarget::Node);
        $crate::scenario!(@__anapao_state_props $connection, $enabled, $metadata, $edges;
            [$($seen)* target]; $($rest)*);
    };
    (@__anapao_state_props $connection:ident, $enabled:ident, $metadata:ident, $edges:ident;
        [$($seen:ident)*]; target: resource_connection($edge:ident), $($rest:tt)*) => {
        $crate::scenario!(@__anapao_state_native_guard target [$($seen)*]);
        $connection = $connection.with_target($crate::types::StateTarget::ResourceConnection(
            $edges(::std::stringify!($edge))?
        ));
        $crate::scenario!(@__anapao_state_props $connection, $enabled, $metadata, $edges;
            [$($seen)* target]; $($rest)*);
    };
    (@__anapao_state_props $connection:ident, $enabled:ident, $metadata:ident, $edges:ident;
        [$($seen:ident)*]; target: state_connection($edge:ident), $($rest:tt)*) => {
        $crate::scenario!(@__anapao_state_native_guard target [$($seen)*]);
        $connection = $connection.with_target($crate::types::StateTarget::StateConnection(
            $edges(::std::stringify!($edge))?
        ));
        $crate::scenario!(@__anapao_state_props $connection, $enabled, $metadata, $edges;
            [$($seen)* target]; $($rest)*);
    };
    (@__anapao_state_props $connection:ident, $enabled:ident, $metadata:ident, $edges:ident;
        [$($seen:ident)*]; target: formula($edge:ident), $($rest:tt)*) => {
        $crate::scenario!(@__anapao_state_native_guard target [$($seen)*]);
        $connection = $connection.with_target($crate::types::StateTarget::Formula(
            $edges(::std::stringify!($edge))?
        ));
        $crate::scenario!(@__anapao_state_props $connection, $enabled, $metadata, $edges;
            [$($seen)* target]; $($rest)*);
    };
    (@__anapao_state_props $connection:ident, $enabled:ident, $metadata:ident, $edges:ident;
        [$($seen:ident)*]; resource_filter: $value:expr, $($rest:tt)*) => {
        $crate::scenario!(@__anapao_state_native_guard resource_filter [$($seen)*]);
        let __anapao_value = $value;
        $connection = $connection.with_resource_filter(__anapao_value);
        $crate::scenario!(@__anapao_state_props $connection, $enabled, $metadata, $edges;
            [$($seen)* resource_filter]; $($rest)*);
    };
    (@__anapao_state_props $connection:ident, $enabled:ident, $metadata:ident, $edges:ident;
        [$($seen:ident)*]; enabled: $value:expr, $($rest:tt)*) => {
        $crate::scenario!(@__anapao_assert_field enabled [$($seen)*]);
        let __anapao_value = $value;
        $enabled = __anapao_value;
        $crate::scenario!(@__anapao_state_props $connection, $enabled, $metadata, $edges;
            [$($seen)* enabled]; $($rest)*);
    };
    (@__anapao_state_props $connection:ident, $enabled:ident, $metadata:ident, $edges:ident;
        [$($seen:ident)*]; metadata {$($key:expr => $value:expr),* $(,)?}, $($rest:tt)*) => {
        $crate::scenario!(@__anapao_assert_field metadata [$($seen)*]);
        $(
            let __anapao_key = $key;
            let __anapao_value = $value;
            $metadata.push((::std::convert::Into::into(__anapao_key),
                ::std::convert::Into::into(__anapao_value)));
        )*
        $crate::scenario!(@__anapao_state_props $connection, $enabled, $metadata, $edges;
            [$($seen)* metadata]; $($rest)*);
    };
    (@__anapao_state_props $($bad:tt)*) => {
        ::std::compile_error!("anapao::scenario!: unknown, duplicate, or malformed state connection property")
    };
    (@__anapao_state_native_guard $field:ident [$($seen:ident)*]) => {
        $crate::scenario!(@__anapao_assert_field connection [$($seen)*]);
        $crate::scenario!(@__anapao_assert_field $field [$($seen)*]);
    };
    (@__anapao_assert_no_native_connection []) => {};
    (@__anapao_assert_no_native_connection [enabled $($rest:ident)*]) => {
        $crate::scenario!(@__anapao_assert_no_native_connection [$($rest)*])
    };
    (@__anapao_assert_no_native_connection [metadata $($rest:ident)*]) => {
        $crate::scenario!(@__anapao_assert_no_native_connection [$($rest)*])
    };
    (@__anapao_assert_no_native_connection [$other:ident $($rest:ident)*]) => {
        ::std::compile_error!("anapao::scenario!: typed connection cannot be mixed with native connection fields")
    };

    (@__anapao_tail $builder:ident, $out:ident, $nodes:ident, $metrics:ident;
        track [$($tracked:ident),* $(,)?]; $($ends:tt)*) => {
        $(
            $builder = $builder.with_tracked_metric(
                $metrics(::std::stringify!($tracked))?
            );
        )*
        $crate::scenario!(@__anapao_top_ends $out, $nodes, $metrics; $($ends)*);
    };
    (@__anapao_tail $builder:ident, $out:ident, $nodes:ident, $metrics:ident;
        $($ends:tt)*) => {
        $crate::scenario!(@__anapao_top_ends $out, $nodes, $metrics; $($ends)*);
    };

    // End-condition parsing. Recursive lists use comma-separated conditions.
    (@__anapao_top_ends $out:ident, $nodes:ident, $metrics:ident;) => {};
    (@__anapao_top_ends $out:ident, $nodes:ident, $metrics:ident;
        end max_steps($value:expr); $($rest:tt)*) => {{
        let __anapao_value = $value;
        $out.push($crate::types::EndConditionSpec::MaxSteps { steps: __anapao_value });
        $crate::scenario!(@__anapao_top_ends $out, $nodes, $metrics; $($rest)*);
    }};
    (@__anapao_top_ends $out:ident, $nodes:ident, $metrics:ident;
        end metric_at_least($metric:ident, $value:expr); $($rest:tt)*) => {{
        let __anapao_value = $value;
        $out.push($crate::types::EndConditionSpec::MetricAtLeast {
            metric: $metrics(::std::stringify!($metric))?, value_scaled: __anapao_value
        });
        $crate::scenario!(@__anapao_top_ends $out, $nodes, $metrics; $($rest)*);
    }};
    (@__anapao_top_ends $out:ident, $nodes:ident, $metrics:ident;
        end metric_at_most($metric:ident, $value:expr); $($rest:tt)*) => {{
        let __anapao_value = $value;
        $out.push($crate::types::EndConditionSpec::MetricAtMost {
            metric: $metrics(::std::stringify!($metric))?, value_scaled: __anapao_value
        });
        $crate::scenario!(@__anapao_top_ends $out, $nodes, $metrics; $($rest)*);
    }};
    (@__anapao_top_ends $out:ident, $nodes:ident, $metrics:ident;
        end node_at_least($node:ident, $value:expr); $($rest:tt)*) => {{
        let __anapao_value = $value;
        $out.push($crate::types::EndConditionSpec::NodeAtLeast {
            node_id: $nodes(::std::stringify!($node))?, value_scaled: __anapao_value
        });
        $crate::scenario!(@__anapao_top_ends $out, $nodes, $metrics; $($rest)*);
    }};
    (@__anapao_top_ends $out:ident, $nodes:ident, $metrics:ident;
        end node_at_most($node:ident, $value:expr); $($rest:tt)*) => {{
        let __anapao_value = $value;
        $out.push($crate::types::EndConditionSpec::NodeAtMost {
            node_id: $nodes(::std::stringify!($node))?, value_scaled: __anapao_value
        });
        $crate::scenario!(@__anapao_top_ends $out, $nodes, $metrics; $($rest)*);
    }};
    (@__anapao_top_ends $out:ident, $nodes:ident, $metrics:ident;
        end condition($condition:expr); $($rest:tt)*) => {{
        let __anapao_condition = $condition;
        $out.push(__anapao_condition);
        $crate::scenario!(@__anapao_top_ends $out, $nodes, $metrics; $($rest)*);
    }};
    (@__anapao_top_ends $out:ident, $nodes:ident, $metrics:ident;
        end any [$($items:tt)*]; $($rest:tt)*) => {{
        let mut __anapao_items = ::std::vec::Vec::new();
        $crate::scenario!(@__anapao_end_items __anapao_items, $nodes, $metrics; $($items)* ,);
        $out.push($crate::types::EndConditionSpec::Any(__anapao_items));
        $crate::scenario!(@__anapao_top_ends $out, $nodes, $metrics; $($rest)*);
    }};
    (@__anapao_top_ends $out:ident, $nodes:ident, $metrics:ident;
        end all [$($items:tt)*]; $($rest:tt)*) => {{
        let mut __anapao_items = ::std::vec::Vec::new();
        $crate::scenario!(@__anapao_end_items __anapao_items, $nodes, $metrics; $($items)* ,);
        $out.push($crate::types::EndConditionSpec::All(__anapao_items));
        $crate::scenario!(@__anapao_top_ends $out, $nodes, $metrics; $($rest)*);
    }};
    (@__anapao_top_ends $($bad:tt)*) => {
        ::std::compile_error!("anapao::scenario!: malformed end condition")
    };

    (@__anapao_end_items $out:ident, $nodes:ident, $metrics:ident;) => {};
    (@__anapao_end_items $out:ident, $nodes:ident, $metrics:ident; , $($rest:tt)*) => {
        $crate::scenario!(@__anapao_end_items $out, $nodes, $metrics; $($rest)*);
    };
    (@__anapao_end_items $out:ident, $nodes:ident, $metrics:ident;
        max_steps($value:expr), $($rest:tt)*) => {{
        let __anapao_value = $value;
        $out.push($crate::types::EndConditionSpec::MaxSteps { steps: __anapao_value });
        $crate::scenario!(@__anapao_end_items $out, $nodes, $metrics; $($rest)*);
    }};
    (@__anapao_end_items $out:ident, $nodes:ident, $metrics:ident;
        metric_at_least($metric:ident, $value:expr), $($rest:tt)*) => {{
        let __anapao_value = $value;
        $out.push($crate::types::EndConditionSpec::MetricAtLeast {
            metric: $metrics(::std::stringify!($metric))?, value_scaled: __anapao_value
        });
        $crate::scenario!(@__anapao_end_items $out, $nodes, $metrics; $($rest)*);
    }};
    (@__anapao_end_items $out:ident, $nodes:ident, $metrics:ident;
        metric_at_most($metric:ident, $value:expr), $($rest:tt)*) => {{
        let __anapao_value = $value;
        $out.push($crate::types::EndConditionSpec::MetricAtMost {
            metric: $metrics(::std::stringify!($metric))?, value_scaled: __anapao_value
        });
        $crate::scenario!(@__anapao_end_items $out, $nodes, $metrics; $($rest)*);
    }};
    (@__anapao_end_items $out:ident, $nodes:ident, $metrics:ident;
        node_at_least($node:ident, $value:expr), $($rest:tt)*) => {{
        let __anapao_value = $value;
        $out.push($crate::types::EndConditionSpec::NodeAtLeast {
            node_id: $nodes(::std::stringify!($node))?, value_scaled: __anapao_value
        });
        $crate::scenario!(@__anapao_end_items $out, $nodes, $metrics; $($rest)*);
    }};
    (@__anapao_end_items $out:ident, $nodes:ident, $metrics:ident;
        node_at_most($node:ident, $value:expr), $($rest:tt)*) => {{
        let __anapao_value = $value;
        $out.push($crate::types::EndConditionSpec::NodeAtMost {
            node_id: $nodes(::std::stringify!($node))?, value_scaled: __anapao_value
        });
        $crate::scenario!(@__anapao_end_items $out, $nodes, $metrics; $($rest)*);
    }};
    (@__anapao_end_items $out:ident, $nodes:ident, $metrics:ident;
        condition($condition:expr), $($rest:tt)*) => {{
        let __anapao_condition = $condition;
        $out.push(__anapao_condition);
        $crate::scenario!(@__anapao_end_items $out, $nodes, $metrics; $($rest)*);
    }};
    (@__anapao_end_items $out:ident, $nodes:ident, $metrics:ident;
        any [$($items:tt)*], $($rest:tt)*) => {{
        let mut __anapao_nested = ::std::vec::Vec::new();
        $crate::scenario!(@__anapao_end_items __anapao_nested, $nodes, $metrics; $($items)* ,);
        $out.push($crate::types::EndConditionSpec::Any(__anapao_nested));
        $crate::scenario!(@__anapao_end_items $out, $nodes, $metrics; $($rest)*);
    }};
    (@__anapao_end_items $out:ident, $nodes:ident, $metrics:ident;
        all [$($items:tt)*], $($rest:tt)*) => {{
        let mut __anapao_nested = ::std::vec::Vec::new();
        $crate::scenario!(@__anapao_end_items __anapao_nested, $nodes, $metrics; $($items)* ,);
        $out.push($crate::types::EndConditionSpec::All(__anapao_nested));
        $crate::scenario!(@__anapao_end_items $out, $nodes, $metrics; $($rest)*);
    }};
    (@__anapao_end_items $($bad:tt)*) => {
        ::std::compile_error!("anapao::scenario!: malformed recursive end condition")
    };

    ($($bad:tt)*) => {
        ::std::compile_error!("anapao::scenario!: expected canonical id/fields/nodes/edges/track/end order")
    };
}
