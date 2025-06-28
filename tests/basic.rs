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
    assert_eq!(5, device.writes_count);
}

#[test]
fn create_file() {
    let device = mounting(device(), |ctrl| {
        assert_eq!(Ok(0), ctrl.count_files());
        assert_eq!(Ok(()), ctrl.create(FILE_NAME, DATA_FIXTURE));
        assert_eq!(Ok(1), ctrl.count_files());
    });

    assert_eq!(26, device.reads_count);
    assert_eq!(28, device.writes_count)
}

#[test]
fn create_then_delete_file() {
    let device = mounting(device(), |ctrl| {
        assert_eq!(Ok(()), ctrl.create(FILE_NAME, DATA_FIXTURE));
        assert_eq!(Ok(1), ctrl.count_files());
        ctrl.print_tree().unwrap()
    });

    println!("======");

    let device = mounting(device, |ctrl| {
        assert_eq!(Ok(()), ctrl.delete(FILE_NAME));
        assert_eq!(Ok(0), ctrl.count_files());
        ctrl.print_tree().unwrap()
    });

    assert_eq!(89, device.reads_count);
    assert_eq!(44, device.writes_count)
}

#[test]
fn create_file_with_long_name_fails() {
    let long_name = str::from_utf8(&[27u8; 129]).unwrap();
    let device = mounting(device(), |ctrl| {
        assert_eq!(Error::FileNameTooLong, ctrl.create(long_name, DATA_FIXTURE).unwrap_err());
    });

    assert_eq!(1, device.reads_count);
    assert_eq!(5, device.writes_count);
}

#[test]
fn create_file_with_data_too_big() {
    let big_data = [255u8; 5121];
    let device = mounting(device(), |ctrl| {
        assert_eq!(Error::FileTooLarge, ctrl.create(FILE_NAME, &big_data).unwrap_err());
    });

    assert_eq!(1, device.reads_count);
    assert_eq!(5, device.writes_count);
}

#[test]
fn create_max_files() {
    let device = mounting(device(), |ctrl| {
        let n_files = 1024;

        let mut nn = 0;
        let mut nnn = 0;

        for i in 0..=n_files {
            let full_dir = i % 26 == 0;

            if full_dir && i > 0 {
                nn += 1;
                if nn == 16 {
                    nn = 0;
                    nnn += 1;
                }
            }

            let file_name = format!("/{nnn}/{nn}/file-{i}");
            println!("Creating file: {file_name}");
            assert_eq!(Ok(()), ctrl.create(&file_name, DATA_FIXTURE));
        }
    });

    assert_eq!(141862, device.reads_count);
    assert_eq!(7524, device.writes_count);
}

#[test]
fn create_file_twice_fails() {
    let device = mounting(device(), |ctrl| {
        assert_eq!(Ok(()), ctrl.create(FILE_NAME, DATA_FIXTURE));
    });
    let device = mounting(device, |ctrl| {
        assert_eq!(Error::FileAlreadyExists, ctrl.create(FILE_NAME, DATA_FIXTURE).unwrap_err(),);
    });

    assert_eq!(27, device.reads_count);
    assert_eq!(28, device.writes_count);
}

#[test]
fn delete_file_that_does_not_exist() {
    let device = mounting(device(), |ctrl| {
        assert_eq!(Error::FileNotFound, ctrl.delete(FILE_NAME).unwrap_err());
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
