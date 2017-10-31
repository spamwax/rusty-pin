mod api;

fn can_access_api(_x: usize) -> Option<usize> {
    api::recent_update();
    None
}