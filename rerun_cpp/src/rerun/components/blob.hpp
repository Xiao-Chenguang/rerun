// DO NOT EDIT! This file was auto-generated by crates/build/re_types_builder/src/codegen/cpp/mod.rs
// Based on "crates/store/re_types/definitions/rerun/components/blob.fbs".

#pragma once

#include "../collection.hpp"
#include "../datatypes/blob.hpp"
#include "../result.hpp"

#include <cstdint>
#include <memory>
#include <utility>

namespace rerun::components {
    /// **Component**: A binary blob of data.
    struct Blob {
        rerun::datatypes::Blob data;

      public:
        Blob() = default;

        Blob(rerun::datatypes::Blob data_) : data(std::move(data_)) {}

        Blob& operator=(rerun::datatypes::Blob data_) {
            data = std::move(data_);
            return *this;
        }

        Blob(rerun::Collection<uint8_t> data_) : data(std::move(data_)) {}

        Blob& operator=(rerun::Collection<uint8_t> data_) {
            data = std::move(data_);
            return *this;
        }

        /// Cast to the underlying Blob datatype
        operator rerun::datatypes::Blob() const {
            return data;
        }
    };
} // namespace rerun::components

namespace rerun {
    static_assert(sizeof(rerun::datatypes::Blob) == sizeof(components::Blob));

    /// \private
    template <>
    struct Loggable<components::Blob> {
        static constexpr std::string_view ComponentType = "rerun.components.Blob";

        /// Returns the arrow data type this type corresponds to.
        static const std::shared_ptr<arrow::DataType>& arrow_datatype() {
            return Loggable<rerun::datatypes::Blob>::arrow_datatype();
        }

        /// Serializes an array of `rerun::components::Blob` into an arrow array.
        static Result<std::shared_ptr<arrow::Array>> to_arrow(
            const components::Blob* instances, size_t num_instances
        ) {
            if (num_instances == 0) {
                return Loggable<rerun::datatypes::Blob>::to_arrow(nullptr, 0);
            } else if (instances == nullptr) {
                return rerun::Error(
                    ErrorCode::UnexpectedNullArgument,
                    "Passed array instances is null when num_elements> 0."
                );
            } else {
                return Loggable<rerun::datatypes::Blob>::to_arrow(&instances->data, num_instances);
            }
        }
    };
} // namespace rerun
