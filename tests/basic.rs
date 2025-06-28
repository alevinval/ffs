use ffs::{BlockDevice, Controller, Error, disk::MemoryDisk};

const FILE_NAME: &str = "/some/path/some-file-name";
const DATA_FIXTURE: &[u8] = b"some data for file";

#[test]
fn mount_device_unsupported() {
    let device = MemoryDisk::new(512, 2048);
    assert_eq!(Error::Unsupported, Controller::mount(device).unwrap_err());
}

#[test]
fn mount_device_formatted() {
    let mut device = MemoryDisk::new(512, 8 * 1024 * 1024);
    Controller::format(&mut device).expect("should format device");
    let sut = Controller::mount(device).expect("should mount on formatted device");
    let device = sut.unmount();

    assert_eq!(1, device.reads_count);
    assert_eq!(1, device.writes_count);
}

#[test]
fn create_file() {
    let device = mounting(device(), |ctrl| {
        assert_eq!(0, ctrl.entries().count());
        assert_eq!(Ok(()), ctrl.create(FILE_NAME, DATA_FIXTURE));
        assert_eq!(1, ctrl.entries().count());
    });

    assert_eq!(3074, device.reads_count);
    assert_eq!(5, device.writes_count)
}

#[test]
fn create_then_delete_file() {
    let device = mounting(device(), |ctrl| {
        assert_eq!(Ok(()), ctrl.create(FILE_NAME, DATA_FIXTURE));
        assert_eq!(1, ctrl.entries().count());
    });

    let device = mounting(device, |ctrl| {
        assert_eq!(Ok(()), ctrl.delete(FILE_NAME));
        assert_eq!(0, ctrl.entries().count());
    });

    assert_eq!(3077, device.reads_count);
    assert_eq!(18, device.writes_count)
}

#[test]
fn create_file_with_long_name_fails() {
    let long_name = str::from_utf8(&[27u8; 129]).unwrap();
    let device = mounting(device(), |ctrl| {
        assert_eq!(Error::FileNameTooLong, ctrl.create(long_name, DATA_FIXTURE).unwrap_err());
    });

    assert_eq!(1, device.reads_count);
    assert_eq!(1, device.writes_count);
}

#[test]
fn create_file_with_data_too_big() {
    let big_data = [255u8; 5121];
    let device = mounting(device(), |ctrl| {
        assert_eq!(Error::FileTooLarge, ctrl.create(FILE_NAME, &big_data).unwrap_err());
    });

    assert_eq!(1, device.reads_count);
    assert_eq!(1, device.writes_count);
}

#[test]
fn create_more_than_max_files() {
    let device = mounting(device(), |ctrl| {
        let n = 1024;
        for i in 0..=n {
            let file_name = format!("/file-{i}");
            if i < n {
                assert_eq!(Ok(()), ctrl.create(&file_name, DATA_FIXTURE));
            } else {
                assert_eq!(Error::StorageFull, ctrl.create(&file_name, DATA_FIXTURE).unwrap_err());
            }
        }
    });

    assert_eq!(1576421, device.reads_count);
    assert_eq!(4097, device.writes_count);
}

#[test]
fn create_file_twice_fails() {
    let device = mounting(device(), |ctrl| {
        assert_eq!(Ok(()), ctrl.create(FILE_NAME, DATA_FIXTURE));
    });
    let device = mounting(device, |ctrl| {
        assert_eq!(Error::FileAlreadyExists, ctrl.create(FILE_NAME, DATA_FIXTURE).unwrap_err(),);
    });

    assert_eq!(1029, device.reads_count);
    assert_eq!(5, device.writes_count);
}

#[test]
fn delete_file_that_does_not_exist() {
    let device = mounting(device(), |ctrl| {
        assert_eq!(Error::FileNotFound, ctrl.delete(FILE_NAME).unwrap_err());
    });

    assert_eq!(1025, device.reads_count);
    assert_eq!(1, device.writes_count);
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
