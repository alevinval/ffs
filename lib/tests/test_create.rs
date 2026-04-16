use common::*;
use ffs_lib::{Error, constants};

mod common;

#[test]
fn given_create_when_empty_file_then_creates() {
    let device = run(|ctrl| {
        assert_eq!(Ok(0), ctrl.count_files());
        assert_eq!(Ok(()), ctrl.create("some/directory/file.txt", &[0; 0]));
        assert_eq!(Ok(1), ctrl.count_files());
    });

    assert_eq!(16, device.reads_count);
    assert_eq!(24, device.writes_count);
}

#[test]
fn given_create_file_with_data_then_creates() {
    let device = run(|ctrl| {
        assert_eq!(Ok(0), ctrl.count_files());
        assert_eq!(Ok(()), ctrl.create("some/directory/file.txt", &[0; 128]));
        assert_eq!(Ok(1), ctrl.count_files());
    });

    assert_eq!(22, device.reads_count);
    assert_eq!(26, device.writes_count);
}

#[test]
fn given_create_when_thousands_of_files_then_creates() {
    let device = run(|ctrl| {
        let n_files = 1024;

        let mut dir = 0;
        let mut subdir = 0;

        for i in 0..=n_files {
            let full_dir = i % constants::TREE_NODE_ENTRY_LEN == 0;

            if full_dir && i > 0 {
                subdir += 1;
                if subdir == constants::TREE_NODE_ENTRY_LEN {
                    subdir = 0;
                    dir += 1;
                }
            }

            let file_path = format!("/{dir}/{subdir}/file-{i}");
            println!("creating {file_path}");
            assert_eq!(Ok(()), ctrl.create(&file_path, &[0u8; 1]));
        }
    });

    assert_eq!(10287, device.reads_count);
    assert_eq!(7439, device.writes_count);
}

#[test]
fn given_create_when_path_too_long_then_fail() {
    run(|ctrl| {
        assert_eq!(Ok(0), ctrl.count_files());
        assert_eq!(
            Err(Error::NameTooLong),
            ctrl.create(
                "some/thisnameexceedsthevalidlengthandshoudlfailthatishowthisworks/file.txt",
                &[0u8; 128]
            )
        );
        assert_eq!(Ok(0), ctrl.count_files());
    });
}

#[test]
fn given_create_when_file_already_exists_then_fail() {
    run(|ctrl| {
        assert_eq!(Ok(()), ctrl.create("some/path/a.txt", &[0u8; 1]));
        assert_eq!(Err(Error::FileAlreadyExists), ctrl.create("some/path/a.txt", &[0u8; 1]));
    });
}

#[test]
fn given_create_when_data_exceeds_max_filesize_then_fail() {
    run(|ctrl| {
        assert_eq!(
            Err(Error::FileTooLarge),
            ctrl.create("some/path/a.txt", &[255u8; constants::MAX_FILE_SIZE + 1])
        );
    });
}
