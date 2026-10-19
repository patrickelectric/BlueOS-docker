# blueos_msgs/msg/EndpointInfo
# One key a service serves, listed in ServiceInfo.endpoints so clients can discover the API.

# One of: job, query, state, event.
string kind
string name
string key
# A .action for a job, a .srv for a query, a .msg for a state or an event.
string interface_type
# The schema text of interface_type.
string schema
