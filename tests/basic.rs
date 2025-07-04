use ffs::{BlockDevice, Constants, Controller, Error, disk::MemoryDisk};

const FILE_PATH: &str = "/some/path/some-file-name";
const DATA_FIXTURE: &[u8] = b"some data for file";

#[test]
fn mount_device_unsupported() {
    let device = MemoryDisk::new(512, 2048);
    assert_eq!(Error::UnsupportedDevice, Controller::mount(device).unwrap_err());
}

#[test]
fn mount_device_formatted() {
    let mut device = MemoryDisk::new(512, 8 * 1024 * 1024);
    Controller::format(&mut device).expect("should format device");
    let sut = Controller::mount(device).expect("should mount on formatted device");
    let device = sut.unmount();

    assert_eq!(2, device.reads_count);
    assert_eq!(5, device.writes_count);
}

#[test]
fn create_file() {
    let device = mounting(device(), |ctrl| {
        assert_eq!(Ok(0), ctrl.count_files());
        assert_eq!(Ok(()), ctrl.create(FILE_PATH, DATA_FIXTURE));
        assert_eq!(Ok(1), ctrl.count_files());
    });

    assert_eq!(22, device.reads_count);
    assert_eq!(26, device.writes_count)
}

#[test]
fn create_then_delete_file() {
    let device = mounting(device(), |ctrl| {
        assert_eq!(Ok(()), ctrl.create(FILE_PATH, DATA_FIXTURE));
        assert_eq!(Ok(1), ctrl.count_files());
    });

    let device = mounting(device, |ctrl| {
        assert_eq!(Ok(()), ctrl.delete(FILE_PATH));
        assert_eq!(Ok(0), ctrl.count_files());
    });

    assert_eq!(52, device.reads_count);
    assert_eq!(41, device.writes_count)
}

#[test]
fn create_file_with_long_name_fails() {
    let long_name = str::from_utf8(&[27u8; 129]).unwrap();
    let device = mounting(device(), |ctrl| {
        assert_eq!(Error::FileNameTooLong, ctrl.create(long_name, DATA_FIXTURE).unwrap_err());
    });

    assert_eq!(2, device.reads_count);
    assert_eq!(5, device.writes_count);
}

#[test]
fn create_file_with_data_too_big() {
    let big_data = [255u8; 5121];
    let device = mounting(device(), |ctrl| {
        assert_eq!(Error::FileTooLarge, ctrl.create(FILE_PATH, &big_data).unwrap_err());
    });

    assert_eq!(2, device.reads_count);
    assert_eq!(5, device.writes_count);
}

#[test]
fn create_max_files() {
    let device = mounting(device(), |ctrl| {
        let n_files = 1024;

        let mut dir = 0;
        let mut subdir = 0;

        for i in 0..=n_files {
            let full_dir = i % Constants::DIR_NODE_ENTRIES == 0;

            if full_dir && i > 0 {
                subdir += 1;
                if subdir == Constants::DIR_NODE_ENTRIES {
                    subdir = 0;
                    dir += 1;
                }
            }

            let file_path = format!("/{dir}/{subdir}/file-{i}");
            println!("creating {file_path}");
            assert_eq!(Ok(()), ctrl.create(&file_path, DATA_FIXTURE));
        }
    });

    assert_eq!(10288, device.reads_count);
    assert_eq!(7446, device.writes_count);
}

#[test]
fn create_file_twice_fails() {
    let device = mounting(device(), |ctrl| {
        assert_eq!(Ok(()), ctrl.create(FILE_PATH, DATA_FIXTURE));
    });

    let device = mounting(device, |ctrl| {
        assert_eq!(
            Error::FileAlreadyExists,
            ctrl.create(FILE_PATH, DATA_FIXTURE).expect_err("should have failed creating twice")
        );
    });

    assert_eq!(23, device.reads_count);
    assert_eq!(26, device.writes_count);
}

#[test]
fn delete_file_that_does_not_exist() {
    let device = mounting(device(), |ctrl| {
        assert_eq!(Error::FileNotFound, ctrl.delete(FILE_PATH).unwrap_err());
    });

    assert_eq!(5, device.reads_count);
    assert_eq!(5, device.writes_count);
}

fn device() -> MemoryDisk {
    let mut device = MemoryDisk::new(512, 8 * 1024 * 1024);
    Controller::format(&mut device).expect("should format device");
    device
}

fn mounting<D>(device: D, mut test: impl FnMut(&mut Controller<D>)) -> D
where
    D: BlockDevice,
{
    let mut ctrl = Controller::mount(device).expect("should mount device");
    test(&mut ctrl);
    ctrl.unmount()
}
