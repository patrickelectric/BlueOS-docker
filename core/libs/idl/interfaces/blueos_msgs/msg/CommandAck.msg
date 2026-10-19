# blueos_msgs/msg/CommandAck
# Reply to a Command, which submits a Job or controls one.

# The STATUS_ values of blueos_msgs/JobStatus. STATUS_UNKNOWN when the Command named no Job.
uint8 STATUS_UNKNOWN=0
uint8 STATUS_ACCEPTED=1
uint8 STATUS_EXECUTING=2
uint8 STATUS_CANCELING=3
uint8 STATUS_SUCCEEDED=4
uint8 STATUS_CANCELED=5
uint8 STATUS_ABORTED=6
uint8 STATUS_WAITING_FOR_PERMISSION=7
uint8 STATUS_WAITING_FOR_RESOURCE=8
uint8 STATUS_PAUSED=9

bool accepted
# The UUID the client generated for the Job, as text.
string job_id
# The status of the Job after the Command was applied, so a Job that ended in that step returns its final status.
uint8 status
string reason
