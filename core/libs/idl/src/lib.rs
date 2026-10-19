//! BlueOS interface types, generated from the ROS 2 interface files in `interfaces/`, and their CDR encoding.

pub mod cdr;
#[allow(clippy::all, clippy::pedantic, dead_code, non_snake_case, unused_imports)]
mod generated;

pub use generated::{MESSAGE_REGISTRY, blueos_example_msgs, blueos_msgs, builtin_interfaces, foxglove_msgs};
pub use roslibrust::RosMessageType;

/// A generated message and its ROS 2 schema name, as in `application/cdr;<SCHEMA_NAME>`.
pub trait Message: RosMessageType {
    const SCHEMA_NAME: &'static str;

    /// The schema text: the definition, then each dependency under a `MSG: <pkg>/<Name>` header.
    fn schema() -> &'static str {
        Self::DESCRIPTION.definition
    }
}
