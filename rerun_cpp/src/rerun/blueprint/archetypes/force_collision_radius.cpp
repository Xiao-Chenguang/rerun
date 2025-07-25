// DO NOT EDIT! This file was auto-generated by crates/build/re_types_builder/src/codegen/cpp/mod.rs
// Based on "crates/store/re_types/definitions/rerun/blueprint/archetypes/force_collision_radius.fbs".

#include "force_collision_radius.hpp"

#include "../../collection_adapter_builtins.hpp"

namespace rerun::blueprint::archetypes {
    ForceCollisionRadius ForceCollisionRadius::clear_fields() {
        auto archetype = ForceCollisionRadius();
        archetype.enabled =
            ComponentBatch::empty<rerun::blueprint::components::Enabled>(Descriptor_enabled)
                .value_or_throw();
        archetype.strength =
            ComponentBatch::empty<rerun::blueprint::components::ForceStrength>(Descriptor_strength)
                .value_or_throw();
        archetype.iterations = ComponentBatch::empty<rerun::blueprint::components::ForceIterations>(
                                   Descriptor_iterations
        )
                                   .value_or_throw();
        return archetype;
    }

    Collection<ComponentColumn> ForceCollisionRadius::columns(const Collection<uint32_t>& lengths_
    ) {
        std::vector<ComponentColumn> columns;
        columns.reserve(3);
        if (enabled.has_value()) {
            columns.push_back(enabled.value().partitioned(lengths_).value_or_throw());
        }
        if (strength.has_value()) {
            columns.push_back(strength.value().partitioned(lengths_).value_or_throw());
        }
        if (iterations.has_value()) {
            columns.push_back(iterations.value().partitioned(lengths_).value_or_throw());
        }
        return columns;
    }

    Collection<ComponentColumn> ForceCollisionRadius::columns() {
        if (enabled.has_value()) {
            return columns(std::vector<uint32_t>(enabled.value().length(), 1));
        }
        if (strength.has_value()) {
            return columns(std::vector<uint32_t>(strength.value().length(), 1));
        }
        if (iterations.has_value()) {
            return columns(std::vector<uint32_t>(iterations.value().length(), 1));
        }
        return Collection<ComponentColumn>();
    }
} // namespace rerun::blueprint::archetypes

namespace rerun {

    Result<Collection<ComponentBatch>>
        AsComponents<blueprint::archetypes::ForceCollisionRadius>::as_batches(
            const blueprint::archetypes::ForceCollisionRadius& archetype
        ) {
        using namespace blueprint::archetypes;
        std::vector<ComponentBatch> cells;
        cells.reserve(3);

        if (archetype.enabled.has_value()) {
            cells.push_back(archetype.enabled.value());
        }
        if (archetype.strength.has_value()) {
            cells.push_back(archetype.strength.value());
        }
        if (archetype.iterations.has_value()) {
            cells.push_back(archetype.iterations.value());
        }

        return rerun::take_ownership(std::move(cells));
    }
} // namespace rerun
