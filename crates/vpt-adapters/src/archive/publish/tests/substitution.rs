use super::*;

fn replace_with_foreign(audio: &Path, staged: &Path) {
    let foreign = audio.join("foreign.m4a");
    std::fs::write(&foreign, b"foreign").expect("foreign");
    std::fs::set_permissions(&foreign, std::fs::Permissions::from_mode(0o600)).expect("mode");
    std::fs::rename(&foreign, staged).expect("replace the staging name");
}

#[test]
fn a_staging_name_replaced_before_placement_publishes_the_verified_bytes() {
    let (_temp, audio, root) = audio();
    let staged = write(&root, ".vpt-staging-swap.m4a", b"verified");
    let target = audio.join("id.m4a");
    let result = exclusive_placing(&root, &staged, "id.m4a", |root, file, from, to| {
        replace_with_foreign(&audio, &staged);
        crate::archive::placement::place(root, file, from, to)
    });
    assert_eq!(result, Ok(Published::Placed(target.clone())));
    assert_eq!(std::fs::read(&target).expect("target"), b"verified");
    assert_eq!(mode(&target), 0o600);
    assert_eq!(std::fs::read(&staged).expect("foreign"), b"foreign");
}

#[test]
fn a_target_created_before_placement_is_never_replaced() {
    let (_temp, audio, root) = audio();
    let staged = write(&root, ".vpt-staging-late.m4a", b"verified");
    let target = audio.join("id.m4a");
    let result = exclusive_placing(&root, &staged, "id.m4a", |root, file, from, to| {
        write(root, "id.m4a", b"other");
        crate::archive::placement::place(root, file, from, to)
    });
    assert_eq!(result, Ok(Published::Exists(target.clone())));
    assert_eq!(std::fs::read(&target).expect("target"), b"other");
    assert_eq!(std::fs::read(&staged).expect("staged"), b"verified");
}

#[test]
fn a_volume_that_cannot_clone_falls_back_to_the_exclusive_rename() {
    let unsupported = |root: &RootDir, file: &File, from: &Path, to: &Path| {
        crate::archive::placement::place_with(root, file, from, to, |_, _, _| {
            Err(std::io::Error::from_raw_os_error(libc::ENOTSUP))
        })
    };
    let (_temp, audio, root) = audio();
    let staged = write(&root, ".vpt-staging-plain.m4a", b"verified");
    let target = audio.join("id.m4a");
    assert_eq!(
        exclusive_placing(&root, &staged, "id.m4a", unsupported),
        Ok(Published::Placed(target.clone()))
    );
    assert_eq!(std::fs::read(&target).expect("target"), b"verified");
    assert!(!staged.exists());
    let staged = write(&root, ".vpt-staging-plain.m4a", b"newer");
    assert_eq!(
        exclusive_placing(&root, &staged, "id.m4a", unsupported),
        Ok(Published::Exists(target.clone()))
    );
    assert_eq!(std::fs::read(&target).expect("target"), b"verified");
    assert_eq!(std::fs::read(&staged).expect("staged"), b"newer");
}
