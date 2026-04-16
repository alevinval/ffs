use ffs_lib::{Controller, Error, testutils::MemoryDevice};

#[test]
fn given_unformatted_device_then_unsupported_device() {
    let device = MemoryDevice::new(512, 2048);
    assert_eq!(Error::UnsupportedDevice, Controller::mount(device).unwrap_err());
}

#[test]
fn given_formatted_device_then_mounts() {
    let mut device = MemoryDevice::new(512, 8 * 1024 * 1024);
    Controller::format(&mut device).expect("controller must format");
    let sut = Controller::mount(device).expect("controller must mount");
    let device = sut.unmount();

    assert_eq!(2, device.reads_count);
    assert_eq!(5, device.writes_count);
}
