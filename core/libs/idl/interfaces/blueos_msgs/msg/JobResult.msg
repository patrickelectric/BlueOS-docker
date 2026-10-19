# blueos_msgs/msg/JobResult
# Event published on blueos/v1/<service>/jobs/<JobType>/result when a Job ends, like the result of a ROS 2 action.

# The Job as it ended: Succeeded, Canceled or Aborted, with its reason.
blueos_msgs/JobStatus job
# The Job type's Job result message, CDR with its encapsulation header. Empty when the Job type declares none.
uint8[] result
