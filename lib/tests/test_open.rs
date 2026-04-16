use common::*;
use ffs_lib::constants;

mod common;

#[test]
fn given_open_when_file_exists_then_opens() {
    let device = run(|ctrl| {
        assert_eq!(Ok(()), ctrl.create("some/file.txt", &[0; 0]));
        let _file_handle = ctrl.open("some/file.txt").expect("must open");
    });

    assert_eq!(10, device.reads_count);
    assert_eq!(17, device.writes_count);
}

#[test]
fn given_open_when_readall_then_reads_contents() {
    let device = run(|ctrl| {
        assert_eq!(Ok(()), ctrl.create("some/folder/file.txt", &[123; 256]));
        let mut file_handle = ctrl.open("some/folder/file.txt").expect("must open");

        let mut buf = vec![0; constants::MAX_FILE_SIZE];
        assert_eq!(Ok(256), file_handle.readall(&mut buf));
        assert_eq!([123; 256], &buf[..256]);
    });

    assert_eq!(24, device.reads_count);
    assert_eq!(26, device.writes_count);
}
