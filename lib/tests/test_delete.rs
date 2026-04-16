use common::*;
use ffs_lib::Error;

mod common;

#[test]
fn given_delete_when_file_exists_then_deletes() {
    let device = run(|ctrl| {
        assert_eq!(Ok(()), ctrl.create("some/file/a.txt", &[0; 1]));
        assert_eq!(Ok(1), ctrl.count_files());

        assert_eq!(Ok(()), ctrl.delete("some/file/a.txt"));
        assert_eq!(Ok(0), ctrl.count_files());
    });

    assert_eq!(50, device.reads_count);
    assert_eq!(46, device.writes_count);
}

#[test]
fn given_delete_when_file_not_found_then_fails() {
    let device = run(|ctrl| {
        assert_eq!(Err(Error::FileNotFound), ctrl.delete("does/not/exist/a.txt"));
    });

    assert_eq!(5, device.reads_count);
    assert_eq!(5, device.writes_count);
}
