# blueos_msgs/msg/JobFeedbackList
# State published on blueos/v1/<service>/jobs/<JobType>/feedback: the latest Feedback of each active Job of the type,
# in the order the Jobs were submitted. A Job leaves it when it ends.

blueos_msgs/JobFeedback[] jobs
