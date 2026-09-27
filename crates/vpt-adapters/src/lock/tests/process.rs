use super::*;

#[test]
fn held_lock_excludes_another_process() {
    let (temp, _state) = test_root();
    let started = Instant::now();
    let mut holder = child("lock_hold", temp.path(), started);
    holder.ready().expect("holder acquired lock");
    finished(child("lock_zero_peer", temp.path(), started));
    holder.send("continue").expect("release holder");
    finished(holder);
}
