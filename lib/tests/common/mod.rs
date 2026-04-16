use ffs_lib::{BlockDevice, Controller, testutils::MemoryDevice};

pub fn run(test: impl FnMut(&mut Controller<MemoryDevice>)) -> MemoryDevice {
    run_on(memory_device(), test)
}

fn run_on<D>(device: D, mut test: impl FnMut(&mut Controller<D>)) -> D
where
    D: BlockDevice,
{
    let mut ctrl = Controller::mount(device).expect("should mount device");
    test(&mut ctrl);
    ctrl.unmount()
}

fn memory_device() -> MemoryDevice {
    let mut device = MemoryDevice::new(512, 8 * 1024 * 1024);
    Controller::format(&mut device).expect("should format device");
    device
}
