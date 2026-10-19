# blueos_msgs/msg/JobFeedback
# The latest Feedback of one active Job, in the jobs/<JobType>/feedback State, like the feedback of a ROS 2 action.

# The UUID the client generated for the Job, as text.
string job_id
# The Job type's Feedback message, CDR with its encapsulation header.
uint8[] feedback
