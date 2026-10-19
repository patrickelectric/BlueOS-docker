//! The endpoints of the example service, `api.lock` records them.

use blueos_idl::{
    blueos_example_msgs::{LevelRequest, LevelResponse, PumpState, SetLevelFeedback, SetLevelGoal, SetLevelResult},
    cdr,
};
use blueos_service::Route;

use crate::Request;

pub static ROUTES: &[Route<Request>] = &[
    Route::job::<SetLevelGoal, SetLevelResult, SetLevelFeedback>(
        "SetLevel",
        "blueos_example_msgs/action/SetLevel",
        |bytes| Ok(Request::SetLevel(cdr::decode::<SetLevelGoal>(bytes)?.level)),
        true,
    ),
    Route::query::<LevelRequest, LevelResponse>("Level", "blueos_example_msgs/srv/Level"),
    Route::state::<PumpState>("pump"),
];
