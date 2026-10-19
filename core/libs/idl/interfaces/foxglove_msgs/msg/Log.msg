# foxglove_msgs/msg/Log
# A log message, as the Foxglove SDK schemas/ros2 defines it. Services publish their tracing output with it.

builtin_interfaces/Time timestamp

uint8 UNKNOWN=0
uint8 DEBUG=1
uint8 INFO=2
uint8 WARNING=3
uint8 ERROR=4
uint8 FATAL=5

uint8 level
string message
string name
string file
uint32 line
